// The native-only validation boundary. Documents and resolvers never cross into Python.
#include <cstdlib>
#include <cstring>
#include <memory>
#include <new>
#include <string>
#include <vector>
#include <sbml/SBMLTypes.h>
#include <sbml/packages/comp/util/SBMLResolver.h>
#include <sbml/packages/comp/util/SBMLResolverRegistry.h>
#include <sbml/packages/comp/util/SBMLUri.h>

LIBSBML_CPP_NAMESPACE_USE

using Lookup = SBMLDocument* (*)(void*, const char*, const char*, int);

namespace {
char* copy(const std::string& text) {
    char* result = static_cast<char*>(std::malloc(text.size() + 1));
    if (result == nullptr) throw std::bad_alloc();
    std::memcpy(result, text.c_str(), text.size() + 1);
    return result;
}

class ClosedResolver final : public SBMLResolver {
    void* context;
    Lookup lookup;
public:
    ClosedResolver(void* context_, Lookup lookup_) : context(context_), lookup(lookup_) {}
    ClosedResolver* clone() const override { return new ClosedResolver(*this); }
    SBMLDocument* resolve(const std::string& uri, const std::string& base = "") const override {
        auto* document = lookup(context, uri.c_str(), base.c_str(), 0);
        return document == nullptr ? nullptr : document->clone();
    }
    SBMLUri* resolveUri(const std::string& uri, const std::string& base = "") const override {
        auto* document = lookup(context, uri.c_str(), base.c_str(), 1);
        return document == nullptr ? nullptr : new SBMLUri(document->getLocationURI());
    }
};

// Own copies before changing the registry. Restore it even when validation throws.
class Resolvers final {
    SBMLResolverRegistry* registry = &SBMLResolverRegistry::getInstance();
    std::vector<std::unique_ptr<SBMLResolver>> saved;
    bool installed = false;
public:
    Resolvers() {
        for (int i = 0; i < registry->getNumResolvers(); ++i)
            saved.emplace_back(registry->getResolverByIndex(i)->clone());
    }
    void install(const SBMLResolver& resolver) {
        installed = true;
        while (registry->getNumResolvers() > 0) registry->removeResolver(0);
        registry->addResolver(&resolver);
    }
    ~Resolvers() noexcept {
        if (!installed) return;
        try {
            while (registry->getNumResolvers() > 0) registry->removeResolver(0);
            for (const auto& resolver : saved) registry->addResolver(resolver.get());
        } catch (...) {
            // Never unwind through Rust. If restoration cannot allocate, stay closed.
        }
    }
};
}

extern "C" {
struct S4hValidationIssue {
    unsigned int rule, severity, line, column;
    char *category, *short_message, *message;
};

SBMLDocument* s4hv_read_file(const char* path, int* status) {
    *status = 0;
    try { return readSBMLFromFile(path); }
    catch (const std::bad_alloc&) { *status = -2; return nullptr; }
    catch (...) { *status = -1; return nullptr; }
}

SBMLDocument* s4hv_read_xml(const char* xml, int* status) {
    *status = 0;
    try { return readSBMLFromString(xml); }
    catch (const std::bad_alloc&) { *status = -2; return nullptr; }
    catch (...) { *status = -1; return nullptr; }
}

int s4hv_location(SBMLDocument* doc, const char* uri) {
    try { doc->setLocationURI(uri); return 0; }
    catch (const std::bad_alloc&) { return -2; }
    catch (...) { return -1; }
}

int s4hv_check(SBMLDocument* doc, void* context, Lookup lookup) {
    try {
        ClosedResolver resolver(context, lookup);
        Resolvers guard;
        guard.install(resolver);
        doc->checkConsistency();
        return 0;
    } catch (const std::bad_alloc&) { return -2; }
    catch (...) { return -1; }
}

unsigned int s4hv_count(SBMLDocument* doc) { return doc->getNumErrors(); }

int s4hv_issue(SBMLDocument* doc, unsigned int index, S4hValidationIssue* out) {
    *out = {};
    try {
        const SBMLError* error = doc->getError(index);
        if (error == nullptr) return -1;
        out->rule = error->getErrorId();
        const auto severity = error->getSeverity();
        out->severity = severity == LIBSBML_SEV_ERROR || severity == LIBSBML_SEV_FATAL ||
                        severity == LIBSBML_SEV_SCHEMA_ERROR ? 2 :
                        severity == LIBSBML_SEV_WARNING || severity == LIBSBML_SEV_GENERAL_WARNING ? 1 : 0;
        out->line = error->getLine();
        out->column = error->getColumn();
        out->category = copy(error->getCategoryAsString());
        out->short_message = copy(error->getShortMessage());
        out->message = copy(error->getMessage());
        return 0;
    } catch (const std::bad_alloc&) {
        std::free(out->category); std::free(out->short_message); std::free(out->message);
        *out = {}; return -2;
    } catch (...) {
        std::free(out->category); std::free(out->short_message); std::free(out->message);
        *out = {}; return -1;
    }
}
}

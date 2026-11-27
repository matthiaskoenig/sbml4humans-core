// The native-only validation boundary. Documents and resolvers never cross into Python.
#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <memory>
#include <new>
#include <string>
#include <vector>
#include <sbml/SBMLTypes.h>
#include <sbml/extension/SBMLExtensionRegistry.h>
#include <sbml/extension/SBMLExtension.h>
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

// Canonical keys come from Rust. Native positions add unreported ListOf owners
// and retain libSBML's depth ordering for elements sharing source coordinates.
int s4hv_positions(SBMLDocument* doc, void* context,
    void (*visit)(void*, unsigned int, unsigned int, unsigned int, unsigned int, unsigned int, int)) {
    try {
        std::unique_ptr<List> elements(doc->getAllElements());
        visit(context, doc->getLine(), doc->getColumn(), 0, 0, 0, 0);
        for (unsigned int i = 0; i < elements->getSize(); ++i) {
            auto* element = static_cast<SBase*>(elements->get(i));
            auto* parent = element->getParentSBMLObject();
            unsigned int depth = 0;
            for (auto* p = parent; p != nullptr; p = p->getParentSBMLObject()) ++depth;
            visit(context, element->getLine(), element->getColumn(), depth,
                parent == nullptr ? 0 : parent->getLine(), parent == nullptr ? 0 : parent->getColumn(),
                dynamic_cast<ListOf*>(element) != nullptr);
        }
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

extern "C" int s4hv_rule(unsigned int code, int package_only, S4hValidationIssue* out) {
    *out = {};
    try {
        if (code < 10000) return 1;
        auto& registry = SBMLExtensionRegistry::getInstance();
        std::string package = "core";
        unsigned int offset = 0;
        for (const auto* name : {"comp", "distrib", "fbc", "qual"}) {
            const auto* extension = registry.getExtension(name);
            if (extension == nullptr) continue;
            const auto candidate = extension->getErrorIdOffset();
            if (candidate <= code && candidate > offset) { offset = candidate; package = name; }
        }
        out->category = copy(package);
        if (package_only) return 0;
        auto accepted = [](unsigned int severity) {
            return severity == LIBSBML_SEV_ERROR || severity == LIBSBML_SEV_FATAL ||
                severity == LIBSBML_SEV_SCHEMA_ERROR || severity == LIBSBML_SEV_WARNING ||
                severity == LIBSBML_SEV_GENERAL_WARNING;
        };
        std::string message;
        unsigned int severity = 0;
        bool found = false;
        if (package == "core") {
            for (const auto& version : {std::make_pair(3u, 2u), std::make_pair(3u, 1u), std::make_pair(2u, 5u)}) {
                SBMLError error(code, version.first, version.second);
                if (error.getCategoryAsString() == "Internal" || error.getMessage().empty() || !accepted(error.getSeverity())) continue;
                message = error.getMessage(); severity = error.getSeverity(); found = true; break;
            }
        } else {
            const auto* extension = registry.getExtension(package);
            const auto index = extension->getErrorTableIndex(code);
            if (index != 0) {
                std::vector<unsigned int> versions;
                for (unsigned int i = 0; i < extension->getNumOfSupportedPackageURI(); ++i)
                    versions.push_back(extension->getPackageVersion(extension->getSupportedPackageURI(i)));
                std::sort(versions.rbegin(), versions.rend());
                versions.erase(std::unique(versions.begin(), versions.end()), versions.end());
                for (auto version : versions) {
                    severity = extension->getSeverity(index, version);
                    if (!accepted(severity)) continue;
                    message = extension->getMessage(index, version, ""); found = true; break;
                }
            }
        }
        if (!found) { std::free(out->category); *out = {}; return 1; }
        out->severity = severity == LIBSBML_SEV_WARNING || severity == LIBSBML_SEV_GENERAL_WARNING ? 1 : 2;
        out->message = copy(message);
        return 0;
    } catch (const std::bad_alloc&) {
        std::free(out->category); std::free(out->message); *out = {}; return -2;
    } catch (...) {
        std::free(out->category); std::free(out->message); *out = {}; return -1;
    }
}

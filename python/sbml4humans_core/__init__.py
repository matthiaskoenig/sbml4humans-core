"""Native Rust report construction, with host-generated request identities."""

import json
import uuid

from . import _core
from ._core import (
    REPORT_CONTRACT,
    SOURCE_CONTRACT,
    __version__,
    _source_json,
    report_json,
)

__all__ = [
    "REPORT_CONTRACT",
    "SOURCE_CONTRACT",
    "__version__",
    "report_json",
    "source_json",
]

if hasattr(_core, "VALIDATION_CONTRACT"):
    VALIDATION_CONTRACT = _core.VALIDATION_CONTRACT
    ValidationSession = _core.ValidationSession
    __all__ += ["VALIDATION_CONTRACT", "ValidationSession"]


def source_json(
    source: bytes,
    *,
    uid: str | None = None,
    location: str = "./model.xml",
    max_expanded_elements: int = 10000,
    limits: dict[str, int] | None = None,
) -> bytes:
    """Build a complete source response from plain/gzip SBML or COMBINE bytes.

    Validation and downloads remain independent application operations. External
    documents resolve only within the supplied archive. No files are fetched.
    """
    options = {
        "uid": uuid.uuid4().hex if uid is None else uid,
        "location": location,
        "maxExpandedElements": max_expanded_elements,
    }
    if limits is not None:
        options["limits"] = limits
    return _source_json(source, json.dumps(options))

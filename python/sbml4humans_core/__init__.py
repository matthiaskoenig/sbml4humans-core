"""Native Rust report construction, with host-generated request identities."""

import json
import uuid

from . import _core
from ._core import (
    ODE_SOURCE_CONTRACT,
    REPORT_CONTRACT,
    _ode_path_json,
    SOURCE_CONTRACT,
    SOURCE_PATH_CONTRACT,
    _source_path_json,
    __version__,
    _source_json,
    report_json,
)

__all__ = [
    "ODE_SOURCE_CONTRACT",
    "ode_path_json",
    "REPORT_CONTRACT",
    "SOURCE_CONTRACT",
    "SOURCE_PATH_CONTRACT",
    "source_path_json",
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


def source_path_json(
    path: str,
    *,
    trusted: bool = False,
    uid: str | None = None,
    max_expanded_elements: int = 10000,
    limits: dict[str, int] | None = None,
) -> bytes:
    """Report a path, admitting local companions only when explicitly trusted."""
    options = {
        "uid": uuid.uuid4().hex if uid is None else uid,
        "maxExpandedElements": max_expanded_elements,
    }
    if limits is not None:
        options["limits"] = limits
    return _source_path_json(path, trusted, json.dumps(options))


if hasattr(_core, "VALIDATION_SOURCE_CONTRACT"):
    VALIDATION_SOURCE_CONTRACT = _core.VALIDATION_SOURCE_CONTRACT

    def validation_source(
        path: str,
        *,
        trusted: bool = False,
        max_expanded_elements: int = 10000,
        limits: dict[str, int] | None = None,
    ):
        """Prepare validation entirely in Rust, retaining per-entry worker results."""
        options = {"maxExpandedElements": max_expanded_elements}
        if limits is not None:
            options["limits"] = limits
        return _core._validation_source(path, trusted, json.dumps(options))

    __all__ += ["VALIDATION_SOURCE_CONTRACT", "validation_source"]


def ode_path_json(
    path: str,
    format: str,
    *,
    location: str | None = None,
    trusted: bool = False,
    simulator: bool = False,
    max_expanded_elements: int = 10000,
    limits: dict[str, int] | None = None,
) -> bytes:
    """Export one admitted model with the shared Rust generator."""
    source = {"maxExpandedElements": max_expanded_elements}
    if limits is not None:
        source["limits"] = limits
    return _ode_path_json(
        path,
        trusted,
        json.dumps(
            {
                "source": source,
                "format": format,
                "entry": location,
                "simulator": simulator,
            }
        ),
    )

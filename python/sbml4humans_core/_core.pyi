REPORT_CONTRACT: str
__version__: str

def report_json(source: bytes) -> bytes:
    """Build report JSON from plain or gzipped SBML; raise ValueError on invalid input."""
    ...

SOURCE_CONTRACT: str

def _source_json(source: bytes, options: str) -> bytes:
    """Build the complete source envelope using serialized per-call options."""
    ...

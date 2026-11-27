REPORT_CONTRACT: str
__version__: str

def report_json(source: bytes) -> bytes:
    """Build report JSON from plain or gzipped SBML; raise ValueError on invalid input."""
    ...

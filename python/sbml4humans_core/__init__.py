"""The native Rust report reader.

The application keeps its current Python report implementation until the full
report parity gate passes for every supported feature and input type.
"""
from ._core import REPORT_CONTRACT, __version__, report_json

__all__ = ["REPORT_CONTRACT", "__version__", "report_json"]

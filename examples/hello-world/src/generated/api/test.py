from dataclasses import dataclass, field
from typing import Callable, Awaitable, Dict, Optional

@dataclass
class TestRequest:
    query_params: Dict[str, str] = field(default_factory=dict)

@dataclass
class TestResponse:
    data: str

TestHandler = Callable[[TestRequest], Awaitable[TestResponse]]

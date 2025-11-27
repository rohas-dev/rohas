from pydantic import BaseModel
from typing import Callable, Awaitable, Dict, Optional

class TestRequest(BaseModel):
    query_params: Dict[str, str] = {}

    class Config:
        from_attributes = True

class TestResponse(BaseModel):
    data: str

    class Config:
        from_attributes = True

TestHandler = Callable[[TestRequest], Awaitable[TestResponse]]

from pydantic import BaseModel
from typing import Callable, Awaitable, Dict, Optional

class TimelineTestFastRequest(BaseModel):
    query_params: Dict[str, str] = {}

    class Config:
        from_attributes = True

class TimelineTestFastResponse(BaseModel):
    data: str

    class Config:
        from_attributes = True

TimelineTestFastHandler = Callable[[TimelineTestFastRequest], Awaitable[TimelineTestFastResponse]]

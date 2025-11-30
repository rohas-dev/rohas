from pydantic import BaseModel
from typing import Callable, Awaitable, Dict, Optional

class TimelineTestVerySlowRequest(BaseModel):
    query_params: Dict[str, str] = {}

    class Config:
        from_attributes = True

class TimelineTestVerySlowResponse(BaseModel):
    data: str

    class Config:
        from_attributes = True

TimelineTestVerySlowHandler = Callable[[TimelineTestVerySlowRequest], Awaitable[TimelineTestVerySlowResponse]]

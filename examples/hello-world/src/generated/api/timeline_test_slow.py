from pydantic import BaseModel
from typing import Callable, Awaitable, Dict, Optional

class TimelineTestSlowRequest(BaseModel):
    query_params: Dict[str, str] = {}

    class Config:
        from_attributes = True

class TimelineTestSlowResponse(BaseModel):
    data: str

    class Config:
        from_attributes = True

TimelineTestSlowHandler = Callable[[TimelineTestSlowRequest], Awaitable[TimelineTestSlowResponse]]

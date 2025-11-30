from pydantic import BaseModel
from typing import Callable, Awaitable, Dict, Optional

class TimelineTestMultiStepRequest(BaseModel):
    query_params: Dict[str, str] = {}

    class Config:
        from_attributes = True

class TimelineTestMultiStepResponse(BaseModel):
    data: str

    class Config:
        from_attributes = True

TimelineTestMultiStepHandler = Callable[[TimelineTestMultiStepRequest], Awaitable[TimelineTestMultiStepResponse]]

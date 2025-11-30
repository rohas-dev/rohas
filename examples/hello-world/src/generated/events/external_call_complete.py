from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class ExternalCallComplete(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

ExternalCallCompleteHandler = Callable[[ExternalCallComplete], Awaitable[None]]

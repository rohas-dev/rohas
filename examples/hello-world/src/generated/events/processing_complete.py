from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class ProcessingComplete(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

ProcessingCompleteHandler = Callable[[ProcessingComplete], Awaitable[None]]

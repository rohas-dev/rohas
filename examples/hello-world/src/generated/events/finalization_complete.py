from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class FinalizationComplete(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

FinalizationCompleteHandler = Callable[[FinalizationComplete], Awaitable[None]]

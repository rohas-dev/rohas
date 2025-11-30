from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class ValidationComplete(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

ValidationCompleteHandler = Callable[[ValidationComplete], Awaitable[None]]

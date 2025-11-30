from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class FastCompleted(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

FastCompletedHandler = Callable[[FastCompleted], Awaitable[None]]

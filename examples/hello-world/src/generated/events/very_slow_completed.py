from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class VerySlowCompleted(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

VerySlowCompletedHandler = Callable[[VerySlowCompleted], Awaitable[None]]

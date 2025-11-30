from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class SlowCompleted(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

SlowCompletedHandler = Callable[[SlowCompleted], Awaitable[None]]

from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class CleanupStep2(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

CleanupStep2Handler = Callable[[CleanupStep2], Awaitable[None]]

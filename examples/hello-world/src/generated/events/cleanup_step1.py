from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class CleanupStep1(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

CleanupStep1Handler = Callable[[CleanupStep1], Awaitable[None]]

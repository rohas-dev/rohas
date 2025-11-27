from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class ManualTrigger(BaseModel):
    payload: str
    timestamp: datetime

    class Config:
        from_attributes = True

ManualTriggerHandler = Callable[[ManualTrigger], Awaitable[None]]

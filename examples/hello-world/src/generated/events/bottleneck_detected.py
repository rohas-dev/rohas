from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class BottleneckDetected(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

BottleneckDetectedHandler = Callable[[BottleneckDetected], Awaitable[None]]

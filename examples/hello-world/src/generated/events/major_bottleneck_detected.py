from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class MajorBottleneckDetected(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

MajorBottleneckDetectedHandler = Callable[[MajorBottleneckDetected], Awaitable[None]]

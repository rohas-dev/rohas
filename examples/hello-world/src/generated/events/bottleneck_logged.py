from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class BottleneckLogged(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

BottleneckLoggedHandler = Callable[[BottleneckLogged], Awaitable[None]]

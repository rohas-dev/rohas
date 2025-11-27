from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable
from ..models.user import User

class UserCreated(BaseModel):
    payload: User
    timestamp: datetime

    class Config:
        from_attributes = True

UserCreatedHandler = Callable[[UserCreated], Awaitable[None]]

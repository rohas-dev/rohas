from dataclasses import dataclass
from datetime import datetime
from typing import Callable, Awaitable
from ..models.user import User

@dataclass
class UserCreated:
    payload: User
    timestamp: datetime

UserCreatedHandler = Callable[[UserCreated], Awaitable[None]]

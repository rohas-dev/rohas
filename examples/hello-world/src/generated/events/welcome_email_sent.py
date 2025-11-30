from pydantic import BaseModel
from datetime import datetime
from typing import Callable, Awaitable

class WelcomeEmailSent(BaseModel):
    payload: dict
    timestamp: datetime

    class Config:
        from_attributes = True

WelcomeEmailSentHandler = Callable[[WelcomeEmailSent], Awaitable[None]]

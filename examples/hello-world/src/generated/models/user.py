from pydantic import BaseModel
from typing import Optional
from datetime import datetime

class User(BaseModel):
    id: int
    name: str
    email: str
    createdAt: datetime

    class Config:
        from_attributes = True

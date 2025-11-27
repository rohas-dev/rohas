from pydantic import BaseModel
from typing import Optional
from datetime import datetime

class CreateUserInput(BaseModel):
    name: str
    email: str

    class Config:
        from_attributes = True

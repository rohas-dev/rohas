from pydantic import BaseModel
from typing import Optional
from datetime import datetime

class TimelineTestSlowInput(BaseModel):
    test: str

    class Config:
        from_attributes = True

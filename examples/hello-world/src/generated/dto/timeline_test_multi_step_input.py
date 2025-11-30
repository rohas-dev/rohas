from pydantic import BaseModel
from typing import Optional
from datetime import datetime

class TimelineTestMultiStepInput(BaseModel):
    test: str

    class Config:
        from_attributes = True

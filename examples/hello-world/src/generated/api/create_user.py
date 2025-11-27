from pydantic import BaseModel
from typing import Callable, Awaitable, Dict, Optional
from ..models.user import User
from ..dto.create_user_input import CreateUserInput

class CreateUserRequest(BaseModel):
    body: CreateUserInput
    query_params: Dict[str, str] = {}

    class Config:
        from_attributes = True

class CreateUserResponse(BaseModel):
    data: User

    class Config:
        from_attributes = True

CreateUserHandler = Callable[[CreateUserRequest], Awaitable[CreateUserResponse]]

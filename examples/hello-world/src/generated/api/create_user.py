from dataclasses import dataclass, field
from typing import Callable, Awaitable, Dict, Optional
from ..models.user import User
from ..dto.create_user_input import CreateUserInput

@dataclass
class CreateUserRequest:
    body: CreateUserInput
    query_params: Dict[str, str] = field(default_factory=dict)

@dataclass
class CreateUserResponse:
    data: User

CreateUserHandler = Callable[[CreateUserRequest], Awaitable[CreateUserResponse]]

from datetime import datetime
from generated.models.user import User
from generated.api.create_user import CreateUserRequest, CreateUserResponse

async def handle_create_user(req: CreateUserRequest) -> CreateUserResponse:
    print(f'Creating user: {req}')
    return CreateUserResponse(data=User(id=1, name=req.body.name, email=req.body.email, createdAt=datetime.now()))

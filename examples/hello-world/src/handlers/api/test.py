from datetime import datetime
from generated.state import State
from generated.models.user import User
from generated.api.test import TestRequest, TestResponse

async def handle_test(req: TestRequest, state: State) -> TestResponse:
    user = User(id=1, name='John Doe', email='john.doe@example.com', createdAt=datetime.now())

    # Explicitly trigger event with custom payload
    state.trigger_event('UserCreated', {
        'id': user.id,
        'name': user.name,
        'email': "hello@world.com",
        'created_at': '2024-01-01T00:00:00Z'
    })

    state.set_payload('UserCreated', {
        'id': user.id,
        'name': user.name,
        'email': user.email,
        'created_at': '2024-01-01T00:00:00Z'
    })

    state.trigger_event('ManualTrigger', {
        'payload': 'Hello, world!'
    })

    state.logger.info('Hello, world!')
    state.logger.error('Hello, world!')
    state.logger.warning('Hello, world!')
    state.logger.warn('Hello, world!')
    state.logger.debug('Hello, world!')
    state.logger.trace('Hello, world!')

    return TestResponse(data="Hello, world!")

import asyncio
from generated.state import State
from generated.events.user_created import UserCreated

async def handle_send_welcome_email(event: UserCreated, state: State) -> None:
    """Event handler for user created - simulates sending welcome email"""
    await asyncio.sleep(0.2)  # 200ms - email sending simulation
    state.logger.info(f'Sending welcome email to: {event.payload.email}')
    state.trigger_event('WelcomeEmailSent', {'email': event.payload.email})

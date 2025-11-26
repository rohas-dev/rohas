from generated.events.user_created import UserCreated

async def send_welcome_email(event: UserCreated) -> None:
    # TODO: Implement event handler
    print(f'Handling event: {event}')

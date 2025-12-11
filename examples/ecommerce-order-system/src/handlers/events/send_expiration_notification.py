from generated.events.order_expired import OrderExpired

async def send_expiration_notification(event: OrderExpired) -> None:
    # TODO: Implement event handler
    print(f'Handling event: {event}')

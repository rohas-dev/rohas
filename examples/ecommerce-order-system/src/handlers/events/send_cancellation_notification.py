from generated.events.order_cancelled import OrderCancelled

async def send_cancellation_notification(event: OrderCancelled) -> None:
    # TODO: Implement event handler
    print(f'Handling event: {event}')

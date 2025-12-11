from generated.events.order_status_updated import OrderStatusUpdated

async def send_status_notification(event: OrderStatusUpdated) -> None:
    # TODO: Implement event handler
    print(f'Handling event: {event}')

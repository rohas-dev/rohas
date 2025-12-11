from generated.events.order_delivered import OrderDelivered

async def send_delivery_notification(event: OrderDelivered) -> None:
    # TODO: Implement event handler
    print(f'Handling event: {event}')

from generated.events.order_shipped import OrderShipped

async def send_shipping_notification(event: OrderShipped) -> None:
    # TODO: Implement event handler
    print(f'Handling event: {event}')

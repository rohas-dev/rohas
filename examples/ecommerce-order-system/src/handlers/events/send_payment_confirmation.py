from generated.events.payment_completed import PaymentCompleted

async def send_payment_confirmation(event: PaymentCompleted) -> None:
    # TODO: Implement event handler
    print(f'Handling event: {event}')

import asyncio
from generated.state import State
from generated.events.payment_completed import PaymentCompleted


async def handle_create_shipment(event: PaymentCompleted, state: State) -> None:
    """
    Create shipment after payment is completed.
    This demonstrates the order fulfillment workflow.
    """
    order_id = event.payload.get('orderId')
    payment_id = event.payload.get('paymentId')

    state.logger.info(f'Creating shipment for order {order_id}')

    # Simulate shipment creation
    await asyncio.sleep(0.8)  # 800ms - external shipping API call

    shipping_id = f"ship_{order_id}_{state.generate_id()}"
    tracking_number = f"TRACK{order_id:06d}"

    state.logger.info(f'Shipment {shipping_id} created with tracking {tracking_number}')

    # Trigger shipment event
    state.trigger_event('OrderShipped', {
        'orderId': order_id,
        'shippingId': shipping_id,
        'trackingNumber': tracking_number,
        'estimatedDelivery': '2024-12-25'  # Mock date
    })

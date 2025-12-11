import asyncio
from generated.state import State
from generated.events.order_status_updated import OrderStatusUpdated
from generated.events.order_shipped import OrderShipped
from generated.events.order_delivered import OrderDelivered


async def handle_send_status_notification(event: OrderStatusUpdated, state: State) -> None:
    """Send notification when order status is updated."""
    order_id = event.payload.get('orderId')
    status = event.payload.get('status')

    state.logger.info(f'Sending status notification for order {order_id}, status: {status}')

    await asyncio.sleep(0.1)  # 100ms - email service call

    # In a real app, send email/SMS notification
    state.logger.info(f'Notification sent: Order {order_id} status changed to {status}')


async def handle_send_shipping_notification(event: OrderShipped, state: State) -> None:
    """Send notification when order is shipped."""
    order_id = event.payload.get('orderId')
    tracking_number = event.payload.get('trackingNumber')

    state.logger.info(f'Sending shipping notification for order {order_id}, tracking: {tracking_number}')

    await asyncio.sleep(0.1)
    state.logger.info(f'Shipping notification sent for order {order_id}')


async def handle_send_delivery_notification(event: OrderDelivered, state: State) -> None:
    """Send notification when order is delivered."""
    order_id = event.payload.get('orderId')

    state.logger.info(f'Sending delivery notification for order {order_id}')

    await asyncio.sleep(0.1)
    state.logger.info(f'Delivery notification sent for order {order_id}')


async def handle_send_cancellation_notification(event, state: State) -> None:
    """Send notification when order is cancelled."""
    from generated.events.order_cancelled import OrderCancelled

    order_id = event.payload.get('orderId')
    reason = event.payload.get('reason')

    state.logger.info(f'Sending cancellation notification for order {order_id}, reason: {reason}')

    await asyncio.sleep(0.1)
    state.logger.info(f'Cancellation notification sent for order {order_id}')


async def handle_send_expiration_notification(event, state: State) -> None:
    """Send notification when order expires."""
    from generated.events.order_expired import OrderExpired

    order_id = event.payload.get('orderId')

    state.logger.info(f'Sending expiration notification for order {order_id}')

    await asyncio.sleep(0.1)
    state.logger.info(f'Expiration notification sent for order {order_id}')


async def handle_send_payment_confirmation(event, state: State) -> None:
    """Send payment confirmation notification."""
    from generated.events.payment_completed import PaymentCompleted

    order_id = event.payload.get('orderId')
    payment_id = event.payload.get('paymentId')

    state.logger.info(f'Sending payment confirmation for order {order_id}, payment {payment_id}')

    await asyncio.sleep(0.1)
    state.logger.info(f'Payment confirmation sent for order {order_id}')

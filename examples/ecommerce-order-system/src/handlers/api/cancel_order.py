import asyncio
from datetime import datetime
from generated.state import State
from generated.api.cancel_order import CancelOrderRequest, CancelOrderResponse
from generated.models.order import Order


async def handle_cancel_order(req: CancelOrderRequest, state: State) -> CancelOrderResponse:
    """
    Cancel an order and trigger OrderCancelled event.
    This will release inventory and process refunds.
    """
    order_id = req.path_params.get('orderId')
    reason = req.body.reason if req.body else None

    state.logger.info(f'Cancelling order {order_id}, reason: {reason}')

    await asyncio.sleep(0.1)  # 100ms - cancellation processing

    # In a real app, fetch order from database and update status
    # For demo, create a mock cancelled order
    from generated.models.order import OrderItem

    order = Order(
        id=int(order_id) if order_id else 0,
        customerId=1,
        items=[
            OrderItem(productId=1, quantity=2, price=29.99, subtotal=59.98)
        ],
        total=59.98,
        status="cancelled",
        paymentId="pay_123",
        shippingId=None,
        createdAt=datetime.now(),
        updatedAt=datetime.now(),
        expiresAt=None
    )

    # Trigger OrderCancelled event - this will release inventory and refund payment
    state.trigger_event('OrderCancelled', {
        'orderId': order.id,
        'customerId': order.customerId,
        'reason': reason,
        'total': order.total
    })

    state.logger.info(f'Order {order_id} cancelled successfully')

    return CancelOrderResponse(data=order)

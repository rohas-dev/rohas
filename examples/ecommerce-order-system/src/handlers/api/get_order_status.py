from generated.state import State
from generated.api.get_order_status import GetOrderStatusRequest, GetOrderStatusResponse
from generated.models.order import Order


async def handle_get_order_status(req: GetOrderStatusRequest, state: State) -> GetOrderStatusResponse:
    """
    Get order status by ID.
    In a real app, this would fetch from database.
    """
    order_id = req.path_params.get('orderId')
    state.logger.info(f'Fetching order status for order {order_id}')

    # In a real app, fetch from database
    # For demo, return a mock order
    from datetime import datetime
    from generated.models.order import OrderItem

    # Mock order data
    order = Order(
        id=int(order_id) if order_id else 0,
        customerId=1,
        items=[
            OrderItem(productId=1, quantity=2, price=29.99, subtotal=59.98)
        ],
        total=59.98,
        status="processing",
        paymentId="pay_123",
        shippingId="ship_456",
        createdAt=datetime.now(),
        updatedAt=datetime.now(),
        expiresAt=None
    )

    return GetOrderStatusResponse(data=order)

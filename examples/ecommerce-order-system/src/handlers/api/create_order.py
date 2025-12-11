import asyncio
from datetime import datetime, timedelta
from typing import List
from generated.state import State
from generated.api.create_order import CreateOrderRequest, CreateOrderResponse
from generated.models.order import Order, OrderItem


async def handle_create_order(req: CreateOrderRequest, state: State) -> CreateOrderResponse:
    """
    Create a new order and trigger OrderCreated event.
    This demonstrates a multi-step workflow with event triggering.
    """
    state.logger.info(f'Creating order for customer {req.body.customerId}')

    # Simulate order creation with validation
    await asyncio.sleep(0.2)  # 200ms - validation

    # Calculate total from items
    total = 0.0
    order_items: List[OrderItem] = []

    for item in req.body.items:
        # In a real app, you'd fetch product price from database
        # For demo, we'll use a mock price
        item_price = 29.99  # Mock price
        subtotal = item_price * item.quantity
        total += subtotal

        order_items.append(OrderItem(
            productId=item.productId,
            quantity=item.quantity,
            price=item_price,
            subtotal=subtotal
        ))

    # Create order with expiration (15 minutes)
    order = Order(
        id=state.generate_id(),  # In real app, use proper ID generation
        customerId=req.body.customerId,
        items=order_items,
        total=total,
        status="pending",
        paymentId=None,
        shippingId=None,
        createdAt=datetime.now(),
        updatedAt=datetime.now(),
        expiresAt=datetime.now() + timedelta(minutes=15)
    )

    state.logger.info(f'Order {order.id} created with total ${total:.2f}')

    # Trigger OrderCreated event - this will trigger payment and inventory handlers
    state.trigger_event('OrderCreated', {
        'orderId': order.id,
        'customerId': order.customerId,
        'total': order.total,
        'items': [{'productId': item.productId, 'quantity': item.quantity} for item in order.items],
        'expiresAt': order.expiresAt.isoformat() if order.expiresAt else None
    })

    return CreateOrderResponse(data=order)

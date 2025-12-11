import asyncio
from generated.state import State
from generated.events.order_created import OrderCreated


async def handle_reserve_inventory(event: OrderCreated, state: State) -> None:
    """
    Reserve inventory for order items.
    This is triggered by OrderCreated event.
    """
    order_id = event.payload.get('orderId')
    items = event.payload.get('items', [])

    state.logger.info(f'Reserving inventory for order {order_id}')

    # Simulate inventory check and reservation
    await asyncio.sleep(0.3)  # 300ms - database query

    # Check each item's availability
    all_available = True
    reserved_items = []

    for item in items:
        product_id = item.get('productId')
        quantity = item.get('quantity')

        # In a real app, check database for stock
        # For demo, simulate availability check
        stock_available = True  # Mock check

        if stock_available:
            reserved_items.append({
                'productId': product_id,
                'quantity': quantity,
                'reserved': True
            })
            state.logger.info(f'Reserved {quantity} units of product {product_id}')
        else:
            all_available = False
            state.logger.warning(f'Insufficient stock for product {product_id}')
            break

    if all_available:
        state.logger.info(f'Inventory reserved successfully for order {order_id}')
        state.trigger_event('InventoryReserved', {
            'orderId': order_id,
            'items': reserved_items
        })
    else:
        state.logger.error(f'Inventory reservation failed for order {order_id}')
        state.trigger_event('InventoryOutOfStock', {
            'orderId': order_id,
            'items': items
        })

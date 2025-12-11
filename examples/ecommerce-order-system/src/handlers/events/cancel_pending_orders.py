from generated.state import State
from generated.events.inventory_out_of_stock import InventoryOutOfStock


async def handle_cancel_pending_orders(event: InventoryOutOfStock, state: State) -> None:
    """Cancel pending orders for out-of-stock products."""
    product_id = event.payload.get('productId')

    state.logger.warning(f'Cancelling pending orders for out-of-stock product {product_id}')

    # In a real app, find and cancel all pending orders for this product

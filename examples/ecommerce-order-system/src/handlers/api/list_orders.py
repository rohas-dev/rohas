from generated.state import State
from generated.api.list_orders import ListOrdersRequest, ListOrdersResponse
from generated.models.order import Order
from generated.models.order_list import OrderList


async def handle_list_orders(req: ListOrdersRequest, state: State) -> ListOrdersResponse:
    """
    List all orders (with pagination in real app).
    """
    state.logger.info('Listing orders')

    # In a real app, fetch from database with pagination
    # For demo, return empty list
    orders: list[Order] = []

    order_list = OrderList(
        orders=orders,
        total=len(orders)
    )

    return ListOrdersResponse(data=order_list)

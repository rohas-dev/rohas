from generated.dto.health_response_dto import HealthResponseDto
from generated.api.health import HealthRequest, HealthResponse
from generated.state import State
from datetime import datetime

async def handle_health(req: HealthRequest, state: State) -> HealthResponse:
    # TODO: Implement handler logic
    # For auto-triggers (defined in schema triggers): use state.set_payload('EventName', {...})
    # For manual triggers: use state.trigger_event('EventName', {...})
    return HealthResponse(data=HealthResponseDto(status="ok", timestamp=datetime.now().isoformat()))

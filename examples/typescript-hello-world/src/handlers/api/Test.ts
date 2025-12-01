import { TestRequest, TestResponse } from '@generated/api/test';
import { State } from '@generated/state';
import moment from 'moment';
import axios from 'axios';

export async function handleTest(req: TestRequest, state: State): Promise<TestResponse> {
  // TODO: Implement handler logic
  // For auto-triggers (defined in schema triggers): use state.setPayload('EventName', {...})
  // For manual triggers: use state.triggerEvent('EventName', {...})
  state.logger.info('Hello, world!');
  state.logger.error('Hello, world!');
  state.logger.warning('Hello, world!');
  state.logger.warn('Hello, world!');
  state.logger.debug('Hello, world!');
  state.logger.trace('Hello, world!');
  const response = await axios.get('https://api.github.com');
  state.logger.info('GitHub API response', { response: response.data });

  return { data: moment().format('YYYY-MM-DD HH:mm:ss') };
}

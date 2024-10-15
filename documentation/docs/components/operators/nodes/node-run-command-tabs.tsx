import { Tabs } from 'nextra/components';
import Mixnodes from '../../../pages/operators/nodes/nym-node/snippets/mixnode-run-tab-snippet.mdx';
import EntryGateway from '../../../pages/operators/nodes/nym-node/snippets/entry-gateway-run-tab-snippet.mdx';
import ExitGateway from '../../../pages/operators/nodes/nym-node/snippets/exit-gateway-run-tab-snippet.mdx';

export const RunTabs = () => {

return (
<div>
  <Tabs items={[
    <code>mixnode</code>,
    <code>exit-gateway</code>,
    <code>entry-gateway</code>
  ]} defaultIndex="1">
      <Tabs.Tab><Mixnodes/></Tabs.Tab>
      <Tabs.Tab><ExitGateway/></Tabs.Tab>
      <Tabs.Tab><EntryGateway/></Tabs.Tab>
  </Tabs>
</div>
  )
}

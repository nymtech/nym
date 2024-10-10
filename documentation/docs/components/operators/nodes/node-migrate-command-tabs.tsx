import { Tabs } from 'nextra/components';
import Mixnodes from '../../../pages/operators/nodes/nym-node/snippets/mixnode-migrate-tab-snippet.mdx';
import Gateways from '../../../pages/operators/nodes/nym-node/snippets/gateway-migrate-tab-snippet.mdx'

export const MigrateTabs = () => {

return (
<div>
  <Tabs items={[
    <code>nym-mixnode</code>,
    <code>nym-gateway</code>
  ]} defaultIndex="1">
      <Tabs.Tab><Mixnodes/></Tabs.Tab>
      <Tabs.Tab><Gateways/></Tabs.Tab>
  </Tabs>
</div>
  )
}

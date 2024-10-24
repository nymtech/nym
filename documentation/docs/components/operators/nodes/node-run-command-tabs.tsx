import { Tabs } from 'nextra/components';
import Mixnodes from 'components/operators/snippets/mixnode-run-tab-snippet.mdx';
import EntryGateway from 'components/operators/snippets/entry-gateway-run-tab-snippet.mdx';
import ExitGateway from 'components/operators/snippets/exit-gateway-run-tab-snippet.mdx';

export const RunTabs = () => {

  return (
    <div>
      <Tabs items={[
        <code>mixnode</code>,
        <code>exit-gateway</code>,
        <code>entry-gateway</code>
      ]} defaultIndex={1}>
        <Tabs.Tab><Mixnodes /></Tabs.Tab>
        <Tabs.Tab><ExitGateway /></Tabs.Tab>
        <Tabs.Tab><EntryGateway /></Tabs.Tab>
      </Tabs>
    </div>
  )
}

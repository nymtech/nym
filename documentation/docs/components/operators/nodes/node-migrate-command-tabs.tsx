import { Tabs } from "nextra/components";
import Mixnodes from "components/operators/snippets/mixnode-migrate-tab-snippet.mdx";
import Gateways from "components/operators/snippets/gateway-migrate-tab-snippet.mdx";

export const MigrateTabs = () => {
  return (
    <div>
      <Tabs
        items={[<code>nym-mixnode</code>, <code>nym-gateway</code>]}
        //defaultIndex="1"
        defaultIndex={1}
      >
        <Tabs.Tab>
          <Mixnodes />
        </Tabs.Tab>
        <Tabs.Tab>
          <Gateways />
        </Tabs.Tab>
      </Tabs>
    </div>
  );
};

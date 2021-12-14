import React from 'react'
import { Switch, Route } from 'react-router-dom'
import { Balance } from '../pages/balance'
import { Bond, Delegate, InternalDocs, Receive, Send, Unbond, Undelegate } from '../pages'

export const Routes = () => (
  <Switch>
    <Route path="/balance">
      <Balance />
    </Route>
    <Route path="/send">
      <Send />
    </Route>
    <Route path="/receive">
      <Receive />
    </Route>
    <Route path="/bond">
      <Bond />
    </Route>
    <Route path="/unbond">
      <Unbond />
    </Route>
    <Route path="/delegate">
      <Delegate />
    </Route>
    <Route path="/undelegate">
      <Undelegate />
    </Route>
    <Route path="/docs">
      <InternalDocs />
    </Route>
  </Switch>
)

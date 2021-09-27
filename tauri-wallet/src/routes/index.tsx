import React from 'react'
import { Switch, Route } from 'react-router-dom'
import { NotFound } from './404'
import { Balance } from './balance'
import { Bond } from './bond'
import { Delegate } from './delegate'
import { Receive } from './receive'
import { Send } from './send'
import { SignIn } from './sign-in'
import { Unbond } from './unbond'
import { Undelegate } from './undelegate'
import { InternalDocs } from './internal-docs'

export const Routes = () => (
  <Switch>
    <Route path="/signin">
      <SignIn />
    </Route>
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
    <Route path="*">
      <NotFound />
    </Route>
  </Switch>
)

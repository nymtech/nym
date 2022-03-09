import React from 'react'
import { TPages } from '../types'

export const RenderPage = ({ children, page }: { children: React.ReactElement[]; page: TPages }) => (
  <>
    {React.Children.map(children, (Child) => {
      if (page === Child?.props.page) return Child
      return null
    })}
  </>
)

import React, { useState } from 'react'

const getItem = (key: string) =>
  document.cookie.split('; ').reduce((total, currentCookie) => {
    const item = currentCookie.split('=')
    const storedKey = item[0]
    const storedValue = item[1]

    return key === storedKey ? decodeURIComponent(storedValue) : total
  }, '')

const setItem = (key: string, value: string, numberOfMinutes: number) => {
  const now = new Date()

  // set the time to be now + numberOfMinutes
  now.setTime(now.getTime() + numberOfMinutes * 60 * 1000)

  document.cookie = `${key}=${value}; expires=${now.toUTCString()}; path=/`
}

export const useCookie = (key: string, defaultValue: any) => {
  const getCookie = () => getItem(key) || defaultValue
  const [cookie, setCookie] = useState(getCookie())

  const updateCookie = (value: string, numberOfDays: number) => {
    setCookie(value)
    setItem(key, value, numberOfDays)
  }

  return [cookie, updateCookie]
}

const getFromLocalStorage = (key: string) => localStorage.getItem(key);
const setInLocalStorage = (key: string, value: string) => localStorage.setItem(key, value);

export { getFromLocalStorage, setInLocalStorage };

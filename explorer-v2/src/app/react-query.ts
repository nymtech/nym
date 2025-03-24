import { QueryClient } from "@tanstack/react-query";

let queryClient: QueryClient | null = null;

export function getQueryClient() {
    if (!queryClient) {
        queryClient = new QueryClient(
            //     {
            //     defaultOptions: {
            //         queries: {
            //             staleTime: 10 * 60 * 1000, // 10 mins
            //             refetchOnWindowFocus: false,
            //             refetchOnReconnect: false,
            //         },
            //     },
            // }
        );
    }
    return queryClient;
}

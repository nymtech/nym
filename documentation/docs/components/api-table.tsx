import React, { useState, useEffect } from "react";

export default function NymDealersAddresses({
  endpoint,
}: {
  endpoint: string;
}) {
  const [announceAddresses, setAnnounceAddresses] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const response = await fetch(endpoint);

        if (!response.ok) {
          throw new Error("Failed to fetch data");
        }

        const jsonData = await response.json();

        const addresses = jsonData.data.dealers.map(
          (dealer: any) => dealer.announce_address
        );

        setAnnounceAddresses(addresses);
        setIsLoading(false);
      } catch (error) {
        setError(error instanceof Error ? error.message : "Unknown error");
        setIsLoading(false);
      }
    };

    fetchData();
  }, [endpoint]);

  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error}</div>;

  return (
    <table>
      <tbody>
        {announceAddresses.map((address, index) => (
          <tr key={index}>
            <a href={address}>{address}</a>
          </tr>
        ))}
      </tbody>
    </table>
  );
}

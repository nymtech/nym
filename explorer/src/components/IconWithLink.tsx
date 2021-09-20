// import React from 'react';
// import { Box, Grid, Typography, Link } from '@mui/material';

// export type IconWithLinkProps = {
//   id: string;
//   text: string;
//   SVGIcon: React.FunctionComponent<any>;
//   apiUrl?: any;
//   linkUrl: string;
//   errorMsg: string;
// };

// export const IconWithLink: React.FC<IconWithLinkProps> = ({
//   id,
//   text,
//   SVGIcon,
//   linkUrl,
//   apiUrl,
//   errorMsg,
// }) => {
//   const [data, setData] = React.useState<any>(null);
//   const [err, setErr] = React.useState<string | null>(null);

//   // APT - unsure what type an async await func should be `:Promise<T>` throwing error.
//   const fetchStats = async (url: string) => {
//     try {
//       const res = await fetch(url);
//       const json = await res.json();
//       if (id === 'val') {
//         return setData(json.result.validators);
//       }
//       return setData(json);
//     } catch (error: any) {
//       return setErr(error);
//     }
//   };
//   React.useEffect(() => {
//     fetchStats(apiUrl);
//   }, []);

//   return (
//     <Grid item xs={12} sm={12} md={4}>
//       <Box sx={{}}>
//         <SVGIcon />
//         <Typography
//           sx={{
//             marginLeft: (theme) => theme.spacing(2),
//             color: (theme) => theme.palette.primary.main,
//           }}
//         >
//           {data?.length || ''}
//         </Typography>
//         <Link href={linkUrl} target="_blank">
//           <Typography sx={{ marginLeft: (theme) => theme.spacing(2) }}>
//             {data?.length && text}
//           </Typography>
//         </Link>

//         {err && (
//           <Typography
//             sx={{
//               marginLeft: (theme) => theme.spacing(2),
//               color: 'red',
//             }}
//           >
//             {errorMsg}
//           </Typography>
//         )}
//       </Box>
//     </Grid>
//   );
// };

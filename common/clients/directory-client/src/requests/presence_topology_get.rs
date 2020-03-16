use crate::presence::Topology;

pub struct Request {
    base_url: String,
    path: String,
}

pub trait PresenceTopologyGetRequester {
    fn new(base_url: String) -> Self;
    fn get(&self) -> Result<Topology, reqwest::Error>;
}

impl PresenceTopologyGetRequester for Request {
    fn new(base_url: String) -> Self {
        Request {
            base_url,
            path: "/api/presence/topology".to_string(),
        }
    }

    fn get(&self) -> Result<Topology, reqwest::Error> {
        let url = format!("{}{}", self.base_url, self.path);
        let topology: Topology = reqwest::get(&url)?.json()?;
        Ok(topology)
    }
}

#[cfg(test)]
mod topology_requests {
    use super::*;
    #[cfg(test)]
    use mockito::mock;
    #[cfg(test)]
    mod on_a_400_status {
        use super::*;
        #[test]
        #[should_panic]
        fn it_panics() {
            let _m = mock("GET", "/api/presence/topology")
                .with_status(400)
                .with_body("bad body")
                .create();
            let req = Request::new(mockito::server_url());
            req.get().unwrap();
            _m.assert();
        }
    }
    #[cfg(test)]
    mod on_a_200 {
        use super::*;
        #[test]
        fn it_returns_a_response_with_200_status_and_a_correct_topology() {
            let json = fixtures::topology_response_json();
            let _m = mock("GET", "/api/presence/topology")
                .with_status(200)
                .with_body(json)
                .create();
            let req = Request::new(mockito::server_url());
            let result = req.get();
            assert_eq!(true, result.is_ok());
            assert_eq!(
                1575915097085539300,
                result.unwrap().coco_nodes.first().unwrap().last_seen
            );
            _m.assert();
        }
    }
    #[cfg(test)]
    pub mod fixtures {
        #[cfg(test)]
        pub fn topology_response_json() -> String {
            r#"{
                   "cocoNodes": [
                     {
                       "location": "unknown",
                       "host": "3.8.244.109:4000",
                       "pubKey": "AAAAAAAAAAEKwAECSqKy8I8KkSYIBSctxRBRxuR61PpAOwK0UQtkeuPRdwusAyaoBbvv1IBWyMEhvbgT4CtgUnGfYH2s06CIJ09lWWvQ0Jkgthq12mG73H9QSTNM8RITlF1X5ax9BV0EK34M5dUncn1uEYzJzcbaLjUarf2bqoy906dtQpppUWDRLJI6ycw7rKKJ4ZNUhgi4KAEGBsSgLqc0zDKs0rArwouZyz4ofoWnY68mdJKrVy6Zqz83DSdc7B2hqqkHX_Bfeb4SwAEFuhRpy4HfcuxwcRI9sIWMo_LVmbk19g1gfMRlBrmZqoEQL6rDApVLZ9eMp-5IQK8WLlZpWf4Zjy7kZolARAyp_rHUQkH4PrDjgoPrKbm6qK_iejYpL7qx28Q3VeInMpwMIMaSbbW9y36sEVtGc2I0Iu5vS0sp8ESiVlQ5NaBz72deZ8oKJJ4IEPPHP99-b0UQX80fVIrNM88mMzKy0bHri9NFlmIG-e0G1cqmw_ry3XWGQkcr1M5RuNa6oX50w5QawAEVxd5FP5bE8bS4x54Csof11sQWUTwMp6Q7_3H7ZCTSlKSqujlOhmfqSHfGPO2sDIYPHDhDzjakZpKAZWWhn_hiR6DfPpomQ01ZYUhVKKSMxz7_VPjsQplP0bZXA2gfnkADUN8UQ0N9g_usIw73r4aZsOviMsRM8oByvsjVfUWc4_HTLSdnQyImFkHz9CiCmrIYL2dYQRePRatWggvBAyeRzntxI4jDqLKiBdi54ZlAKgV6MCRaJ7Bu7BtmLXrtK4sawAED3QYxuvOSZrbZdUr4yG-U9yVvJ9Klkf-5Mo4EYp3qTL2KBB6_LrZepjAQqp486YkZ03mTIezcsZ48EboXVTWKBZ3QnTI5tX-j4gGxQb7klOJc97qJkDxsvpz4F0ChgCUIZhpIItWHia7_R3Gi-b5siLIdQdUho9isn3kiDGm6t0NED2Bgy3ZxxQwzqsBZm4kPr2_fPX4YyvIoP9895YcGjZyE5iiRC_TE41RJmB1GZYdxegTMq3lNDllKgiqaiPgawAEJASDkmZHTwlg9YOev5OWpQD-FnhPkqVNo_QcDyRu9eoGcWSGFp2sYqjG2SpmiXq0VNnAO7AcKxRzDFu7TjfhlU3Kt0uTKIcrWVU1zFNbJNMjYEq90pp50nowwx8INz20IXET2ZNX6kIXYFCsEvPLZFlG2OoL6xg3uQS1qMl3lIS_VxdO_JfVe0rT65WsJ_P4Nkc1jYiuNPHY6d_iFO0BVYqX0sOCX73GC_TT13BR0jnPwDAVw0rGtYHsXBb8TKOsawAEZIClauuT1V3qOZnb7uRZhFXO-PKTxgc1LCzJt2ChOrMZaBpjlkf3IPpJ2UF4JH4kGaDeBf2k_S-FLAs3drK21efbi5P6_a4QTxAiiRimXGoQIyvOg462s6kP_ZRFufo8YYQHS4olaOeqU4564dNskg_uBPsFMz_2GNOhmn_15cJqP1jfkyD49Z16GTS5YLHgVl9bJKqyvLuypsToLbt1BJzipEP0L2OohuRm-_MvqvwwWKyjNQsubgee1K728d9AawAEBkGggcNVCtXyhoSqi3_w0tVxtkAYeud8sBeAtZHGs06me_QL8co0MFLlO-zdkUb4ZBq08rFEbgLOma8_3whleM8NIPaHNISp1q3IsIhB5zdXcZoGsqLixODBFHtID3YEHAlr4f9T_yh11yJ95xGCl_6Y37hpwLQVGyrfSfccM24mVFqnV3TT5Wdq3ile-jesUx1Q2G1yK_xVqc6itmk-kDuBjyZgzYi1-jsIXAjnhM9G7t8J_Bv5yGGZhLK2dCzM=",
                       "type": "validator",
                       "lastSeen": 1575915097085539300,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "3.9.129.61:4000",
                       "pubKey": "AAAAAAAAAAMKwAECSqKy8I8KkSYIBSctxRBRxuR61PpAOwK0UQtkeuPRdwusAyaoBbvv1IBWyMEhvbgT4CtgUnGfYH2s06CIJ09lWWvQ0Jkgthq12mG73H9QSTNM8RITlF1X5ax9BV0EK34M5dUncn1uEYzJzcbaLjUarf2bqoy906dtQpppUWDRLJI6ycw7rKKJ4ZNUhgi4KAEGBsSgLqc0zDKs0rArwouZyz4ofoWnY68mdJKrVy6Zqz83DSdc7B2hqqkHX_Bfeb4SwAEEv6RMevAQmLGkeK0uJKnMPPAtm8GgXjWSQijYdnxlPh5SJSNeJUbPZKWFFWdk8yIFXKa8jnzETtdGFKgUUt5AVUDpTBmEdwaHCzlFhXrttshy0V5OhPUlV8cGABmxbagMYm0bFPg0r-snSkrB9YG6wqJYQVeIMOCGYCPbHmDA8R_0-h8VkRKWs1d9KvQOK4kShqgZtYN71KJW8uDE4q2jsGDVvxFt1AgmU9b93xsXF17KrpZy5WxlLZ73HtnTD_oawAED4vd_rK-Kx_n8x_OdDiiEOPUlYDlDCQUqenU9XHKH3B6ijfkJ368wd3LDDVStjDwNORrAyUSw_VlSNUpd1XLC8d17gTaIq5ZI2fWuwwZaoN1JCsYU8fQ6USgtIehQX7IPP8EkFuNmuCBCmpr4schtYniGe9J8Q4dsV-TYPr2uLJkdx1r7luzF--I22k7NfQQM14QDci_0kgrgmZ54CJGkjXyOhCppBXg3fqLC6aFvT3ZocfiiXBJt0huGgPMDtYsawAECLh8KUdNsDolERwJ8v04bS5jI_KKf7uUnCHWuCELwbJSUI3OK1ufS1qSpauvSzVQSbrhEzrEfwQn4VtxQxJlX4UdDU-R-hafiZvVC6DLLAbuORBAC3FScn9W58CnezH4DvCp_w7nftDfdxeuungbZT9XaxS3iNC6PnFsWF6WM3DxMwrzOrFe6wEEoTSPe1mcUDrtwM5UksIvJr6MBRAXrdl0IdBTQr7cLwKe_KYi4siwdjfJEJtOh7oxQBxBg2UkawAEJAPZK2Gg2MQwpxdDT24lNQHF7FVfkO_LuhJwn0RbwNDSVeA4P6-tWL5TkCpqr8xYHfwQ6Z3ILfpGCZr8PspwIoRzqZHQ16f8Pq9xnr0hLEI9BOQU0FS2EtuyPgju5iwsAJAfehUzu6kNLphuLGsXoIZdXDG5mbylwh9JzAVXTwgaR0hNqyXVJxgbt7jcYaSEBFcMGV-hjXyVVNzBleE-G9o_noI_KWU4Ce7K-qOMcewMKfy_VEw-gVaD6dHz6AMoawAEE9XuOLwRttvKybAssZ9gsK-_YRUwuFOeRDIr3NX___9bx6pCc18adCIlH_8EJWFwXZ05ZpNNE88mYx7ZQ3aqaArZJRoWeZeKhqH_s05V10xbzkYX71G5cqz--8vr9ZlQRb2BeETF_Tdq_PLk7qbT8WTGIoq7ZwyDRQTgzvkCgyzj_hBLh2o7sSVNgUo38SFUTMn7YtvVFYlSrTDE3WKE-T-nh5SWdDBxgDTc3Bw8JpzNH-WkoJ4Lim7sB4Op1gEUawAEW4-kenlffwsNr_3b3aV0YuusLpxB03sxPzQ5B0CWNiVtbja1Z4tWhKGUUrdq_eUgMV0y5Of-BqNi5FspAQnhJBFSSxtOzRGV1h3qyUTksfZyed9z8zPI-ZPP9XXm7hYgJgDz_kxte-NfS9UG9q5AZetHUN4kGxXutjjzfUQZ9yTvhBKgKgTI2Dp_R_jZrWQ8F1BoWzIJzjddT1K2MvCQEkARYw08isbOeFmCwgVUcjxYZO45WyOmLQA7QJRL9WvA=",
                       "type": "validator",
                       "lastSeen": 1575915097388409000,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "3.9.222.1:4000",
                       "pubKey": "AAAAAAAAAAQKwAECSqKy8I8KkSYIBSctxRBRxuR61PpAOwK0UQtkeuPRdwusAyaoBbvv1IBWyMEhvbgT4CtgUnGfYH2s06CIJ09lWWvQ0Jkgthq12mG73H9QSTNM8RITlF1X5ax9BV0EK34M5dUncn1uEYzJzcbaLjUarf2bqoy906dtQpppUWDRLJI6ycw7rKKJ4ZNUhgi4KAEGBsSgLqc0zDKs0rArwouZyz4ofoWnY68mdJKrVy6Zqz83DSdc7B2hqqkHX_Bfeb4SwAECh9xcxpjOp1r7kiNIgrI9GgAlvXwgHkTchOxUiyOzTq6FDWdGN64KiC3NDeyGTg8FmzvGzS3jREeJqOdr4G9ZGtWkauAITgLFiH62t-YntRslhr8_1shxlmzKiNKJN_QFflEq79pZIlWtp3N8LIHMvXRtl-zt2DMze4s02XDmEkviyVE4CkQUDtCc-2MfPT4JcmEFqtFIxjrXn18SbYg3c6XUQHsGIkuDrKuCTRlpC8kvmM0uVoIeWdmwDlZk4jUawAEJhRwK5ozjqIWRP1bFzBPS9VhaJnfKU9PeFYtN5beiAHrYr2ylIB3yDfmAQUdKDowDUm5nfJATejEjEnrTGxh70QtfoNV391rSns3F71tBwY62KLaNr8qnVfeSFHV3FcQTMHHF_8mDb5_11Rj6aiMvW0y6eetHo7CDPMdEyDPmok_U2ZM5BzOUnwjT21HtnvcKxKKwHJ_QGfnAHPyDIhNOMgxJCrVazOidLCHeYGpyCLw1ipeTyKOQX0_ByB8dH6AawAEGV1GuF5SSlT67B1ityPJK2ZwXjeeKB4gGdCG3qRtWxLTZfGhVm7YAYm2f5tw_wrsJAZ9FubVhateGg0ZN67NxZtsvOOejXz6743f7ijnQopPgd_8pH-iVf6BEcSO8ZdcHxNRUTayzjVLs99bwMo2zaPevW4X4G_bN4mh---aPkdGYHwaiklzUhqJ-eqycrYAFyjyEXaPBXLQm1rpczqluNvnKbd8Q9LZWukgm7_uWv_HxufIvdWgoq8bAt78UU3oawAEP9VDehhqrQG5-WHMB66XVxo1TgMM8aVV0SwAq3lCRkpiFBz_9kw8T1F9Hx2AiNrEGT1QLbdMkpms1cG_5gBBahQofdt_NmUs1jfTFXY9iyMy1Q7A6ZYaLP8Z6q-orc1cKqySY-BJZQ_CpGFfXS0OVniFDQ6v78ytPK7K-yRgT1PxFgm3rZqrG0Tjbrpsg2PUL5S5fuXfMhUosP0uoLj0D1guWAR9Y7kfFBIXaTSFMoa8fghVBUTRNhK9f72a8SxQawAEOiv71taLjKqaaWQ_QjcDhWbvjG1EnsCyI0toNjGkcF19x4Vk-5NC96_4ioUGz404IC0XN03roRnibRT_78D9vZFVCWCqve9EjdF5TcApx03zIP4JT2g2q0MKIGgGrwt4Pz6LO6yOfMm7B8Yraps8IV-nP1w7K1m9XKP_FvH8egl5GHJe-_omlC2YyL_b28jMLENbxDFD-3KPjZFBhSLrRukX2PlayYTwEiTtokA2R9_11vQvJgP8KFEjGHg6zsAMawAEBn2H_hz2knb8ltnpEA5YSKVcV3nUtojkCNi_WUz7xUKd7efw1oI_lbnKrS7HkyC0JkQUZ1pCWUlSXNmgjMEhsn823a1LFzpV7rOv4vayYvvFX61hB9R78VjpyxJiYpDwRZLiUY3AK4WY8NqFDbjXR7rT4CkFHEf-VhSQQ8ZNvlpod1nmeVQVizHH9e7Tq7wsWz-LWEk3Hx6LmcrgDsL79LZYG9JXU5IdvG8RvLNx9cSwEI8yxcchpISAaot7UoYQ=",
                       "type": "validator",
                       "lastSeen": 1575915094734973000,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "3.9.102.214:4000",
                       "pubKey": "AAAAAAAAAAIKwAECSqKy8I8KkSYIBSctxRBRxuR61PpAOwK0UQtkeuPRdwusAyaoBbvv1IBWyMEhvbgT4CtgUnGfYH2s06CIJ09lWWvQ0Jkgthq12mG73H9QSTNM8RITlF1X5ax9BV0EK34M5dUncn1uEYzJzcbaLjUarf2bqoy906dtQpppUWDRLJI6ycw7rKKJ4ZNUhgi4KAEGBsSgLqc0zDKs0rArwouZyz4ofoWnY68mdJKrVy6Zqz83DSdc7B2hqqkHX_Bfeb4SwAEOVCUN3EwiVroS5-TOq2o7hYSxphK9X0G23N-IBZ0Tr1Rl8XEiJ-OEy0rqnAKwmhAZJWnx3u8oXqbZtOWIZmzQSpcoxhgwfhdmTZJCqT2RVzZyeFItX4sVeilEP3z2xdsJs8-a1kg6UZnx1s1BNLBo7eZrreZygWojPCIDBn03fSAflXoVc5PpY2CGy5MA_IgWgSYBHDdoZEtigp_amjqK7Us44Db20XpLxMXfbahiqa7WKNnMgi6Ca2H67VtaaD8awAEF3zbE1nZRAa7a8vbU25c80YBYJBaW8P6FwXQI-K0Xk5MakwYeMMnIrm6w6IS_0XAO5YlD453GLqnxY8H1BEnRpfOnT7PE4el9mJ8MuYQMo6R2up0lGCmYM0YA9FORjroM3ng69SEPfJPCReG7LfJkERl_m2U403ertDRBYrlqCDagDfyI500srBcMrjSvV3oNouyyx3yZUrjLQfbHhDteQFsYdmakJs8Y-Q9-5MXCcrz6Qa4xwv522Euv0CCxkHcawAEYjfsU_zDhUZA1ey1aquWXlFOnx-iEALqxW1slDYHwQ1M2SILc-v_E6i1doa5e_bAZHVezBHFAlaNAVedNyHFFJxYAqAK3hbzbvl2glw3Q6h_rTXElymloqtaqVFIJ-oUWWOHsZBmu8EDA-HzvGCiBa_GbRaVfh2lE4ObeMXoJrEm_5dbxxeEic2l3IYeIz40N9ooQQOkQcOZdY4AXWYCavIAwWEJBjLtptJgCLu9a_zM1S5GsiyJHpdDs46WbP0EawAEWZ-95Sf0YAHujxRNLdXgpqe0ZF8loVwzZfvyMvqaxF1Ug274BqHuY_c5NdPAzuqoTwjfEn8NKEoaNqlumM75FUYbaTd7mXvk4WVYWjVnkO40dfQjRB7DYhvj0LBlbndAJ4wJIA2ilPYgjZsXVbNNh3e2j3u9eABd0VaFMbSb8Sz5_31r8HzoWmPJs3HiyuyANGFUA6CvAnMN6K3b-D8BhFZU_nPUTgu80o8_n6LQt-XWbaC_mTHzsnOjzBiPJxlYawAEW3bmOEtStH2T8q7vMkhchImp2-hg9MFYGBmEe9sSByTn3NUf8eksqXOC1dUjHkXoZm298FgUYLkNdnlxWpf993j5mEDoFxjcTB7scBD7k6nu6Nrs_wK0-seS8gsHrx9UK7GwAsi10q82Cm4PFyAtrWjmy_d9WLHuZt6VIOKunTs8cf0FwNUiMcvZsruqIFJcP7iWxdiFdUkh65P_iCz1ZEjJcj2GEZoq4v3a3by1aizGPaaiKc1jd_T-XJg_YpncawAEWnstu5b9WiZv0x8xfsiMk6YRlU0Cnj5svxLLXz_8drvwAa--GBY5yH0ke2EM6udMEi2EPeFcGTe6Sjs0YEhSbY7Uad_8suD2J4tIWJSWBbiyvh7rSqzv57m7BlsVcHfQJn_wNH-UlC9xkx8vg-LwfN8_FlxvHNPTc7XZG3lKYbwpUWlZxAziOYT1VQ-2K2bQQBBMdix-ht_SjccL1Dc2dP5kDazQ8yZV_8xnyeheazEedWe63uutfkHlZRg9YwP8=",
                       "type": "validator",
                       "lastSeen": 1575915094967382800,
                       "version": "0.1.0"
                     }
                   ],
                   "mixNodes": [
                     {
                       "location": "unknown",
                       "host": "35.176.155.107:1789",
                       "pubKey": "zSob16499jT7C3S3ky4GihNOjlU6aLfSRkf1xAxOwV0=",
                       "layer": 3,
                       "lastSeen": 1575915096805374500,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "18.130.86.190:1789",
                       "pubKey": "vCdpFc0NvW0NSqsuTxtjFtiSY35aXesgT3JNA8sSIXk=",
                       "layer": 1,
                       "lastSeen": 1575915097370376000,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "3.10.22.152:1789",
                       "pubKey": "OwOqwWjh_IlnaWS2PxO6odnhNahOYpRCkju50beQCTA=",
                       "layer": 1,
                       "lastSeen": 1575915097639423500,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "35.178.213.77:1789",
                       "pubKey": "nkkrUjgL8UJk05QydvWvFSvtRB6nmeV8RMvH5540J3s=",
                       "layer": 2,
                       "lastSeen": 1575915097895166500,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "52.56.99.196:1789",
                       "pubKey": "whHuBuEc6zyOZOquKbuATaH4Crml61V_3Y-MztpWhF4=",
                       "layer": 2,
                       "lastSeen": 1575915096255174700,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "host": "3.9.12.238:1789",
                       "pubKey": "vk5Sr-Xyi0cTbugACv8U42ZJ6hs6cGDox0rpmXY94Fc=",
                       "layer": 3,
                       "lastSeen": 1575915096497827600,
                       "version": "0.1.0"
                     }
                   ],
                   "mixProviderNodes": [
                     {
                       "location": "unknown",
                       "clientListener": "3.8.176.11:8888",
                       "mixnetListener": "3.8.176.11:9999",
                       "pubKey": "54U6krAr-j9nQXFlsHk3io04_p0tctuqH71t7w_usgI=",
                       "registeredClients": [
                         {
                           "pubKey": "zOqdJFH49HcgGSCRnmbXGzovnwRLEPN0YGN1SCafTyo="
                         },
                         {
                           "pubKey": "fy9xo69hZ2UJ9uxhIS1YzKHZsH8saV-02AiyCNXPNUc="
                         },
                         {
                           "pubKey": "6hFCz42d5AODAoMXqcBWtoOoZh-7hPMDXbLKKhS7x3I="
                         },
                         {
                           "pubKey": "pPZMDzw0FyZ-LBAhwCvjlPGj1p2bO_vaWbRBc7Ojq3Q="
                         },
                         {
                           "pubKey": "kjl5poXt2GdIrHLuMBjmTBSChV-zMqsXFBhKZoIREVI="
                         },
                         {
                           "pubKey": "T1jMGuk7-rcIUWnVopAsdGzhAJ7ZVymwON2LzjuOfnY="
                         },
                         {
                           "pubKey": "3X0eKPkflx5Vzok0Gk0jm5-YEfvPAuWP55ovyBUOtXA="
                         },
                         {
                           "pubKey": "5wC3i8rCZolLKbWJ9U6eLieXNGLKM21dtL6lR30u_hE="
                         },
                         {
                           "pubKey": "1P6p7fgjwjlEepD9JgbN1V-rk9n36-hCmPN5P6y62n4="
                         },
                         {
                           "pubKey": "aBjlLQKKFroqhX_kvYLnMm1uq3FJdQWqVy9Q35zzERA="
                         },
                         {
                           "pubKey": "I0gVOPj6lv9ha60xPYKeAgbeUU8pdyMD-Y7Nb1nS9EQ="
                         },
                         {
                           "pubKey": "WgdvQ74QH1uFDWDL2YeApvv7oniNGh9BQJ_HZam20QA="
                         },
                         {
                           "pubKey": "Mlw23KaSL2hyrIjEZM76bZStt2iMzxVAqXwO5clJfxg="
                         },
                         {
                           "pubKey": "F9xzbjnMQVN4ZidcqN2ip9kVnI9wbS39aVayZGiMihY="
                         },
                         {
                           "pubKey": "s6pfVkZrUG--RNjfzS55N2oPvFkMdvgb1LUut6gqRy4="
                         },
                         {
                           "pubKey": "bSi-9k0jJNKc8PGx8M3SWFaNpORFjYw-NkWXRZVRWGU="
                         },
                         {
                           "pubKey": "pz6ahQcGOQcZBFx1tGmzRngqk6BecXB_wFd4WVdOQDU="
                         },
                         {
                           "pubKey": "5sfwIMcG2zRCxhDh4D9Evw1WPI5bfKZAShM_6o9Pu1I="
                         },
                         {
                           "pubKey": "9fZnxXy9onGPpZ3Ygckqw0okqCw3di02sLr-NTBr4SE="
                         },
                         {
                           "pubKey": "Q0TbbggOwzZjalUdi5eEHVFi9VMv-rMm5mJPUZZs12A="
                         },
                         {
                           "pubKey": "aPNyox_qAIGFB2-wZ0lc9iAWDN6jzLojApSiWVFjCks="
                         },
                         {
                           "pubKey": "DUKLEsIGMw-ucs3DjS7Ag9qCb5-C_A84DuIsZuLkdwI="
                         },
                         {
                           "pubKey": "YV84vPoSrLf1p9Sw6FnnrcCpS3kvpJUfKyKpnwk8z0Q="
                         },
                         {
                           "pubKey": "_IYEzZQoBAYeTxqzpEe_ez1-7pn7aId8AKliazy0qlE="
                         },
                         {
                           "pubKey": "srkOAoVU-a02lnEsoH_wOLLw7HFx_xHIZUSbnhjFwDw="
                         },
                         {
                           "pubKey": "LxSCBn_OEQq-hI2xDi6bfGoioRO_lSTIq6AQ9l1k5jg="
                         },
                         {
                           "pubKey": "OC0OOqtGfAytgZjthpjoKeYNa0VrtzfgZ0iO5Fag5y8="
                         },
                         {
                           "pubKey": "ImVEch2focRhm1ial1gA6YJPr6WDyW3oh-OgYcO5Ll4="
                         },
                         {
                           "pubKey": "yTDnzLvEaSq0mC9xNrtyjpAKtsIU6yRuBepCuWQMBm4="
                         },
                         {
                           "pubKey": "-LCUUc46HuL7iUEOMkrlVAkvvulRiQ7dR1QYh7bkKxw="
                         },
                         {
                           "pubKey": "Bx1MGpISig25rqe7mhoX68EROUPPzmF7yGLYah9DPgM="
                         },
                         {
                           "pubKey": "Z4Nu6iwLmgJ93yoIFTbTEBeDAHRwS-vo1T_K2Kv1FQo="
                         },
                         {
                           "pubKey": "cKFAGxllwAmEXCtDxG_T1iEm3-lKWUVQxxpDBje6mQU="
                         },
                         {
                           "pubKey": "pQV40whlQWUSXtrNTTePzO6sdq3zr1JUIWZWvD443nY="
                         },
                         {
                           "pubKey": "6Bb5HwnVqJPy5wcNsaHY-0y__coZCE7XC80kUkesnRU="
                         },
                         {
                           "pubKey": "COGdpfhmzNGR6YX820GqJIkjOihL8mr6-h-d3JlTDFA="
                         }
                       ],
                       "lastSeen": 1575915097358694100,
                       "version": "0.1.0"
                     },
                     {
                       "location": "unknown",
                       "clientListener": "3.8.176.12:8888",
                       "mixnetListener": "3.8.176.12:9999",
                       "pubKey": "sA-sxi038pEbGy4lgZWG-RdHHDkA6kZzu44G0LUxFSc=",
                       "registeredClients": [
                         {
                           "pubKey": "UE-7r6-bpw0b4T3GxOBVxlg02psx23DF2p5Tuf-OBSE="
                         },
                         {
                           "pubKey": "UnZuLpzq64_EPtIcr1Fd-5AESBCBLFnDMDsjUaOqrUA="
                         },
                         {
                           "pubKey": "4ExXPrW5w0nZIQ5ravBRT6H9r0RH0MXuOcGIF8HzUhg="
                         },
                         {
                           "pubKey": "wPsRpJPi1e2sjItyRKDkFACbxwu3Cw5GlYVPmdYxk2U="
                         },
                         {
                           "pubKey": "UfK5UvT3HUkT1SGbv1QGafy3in3uQ9a6NSy5EOT6k0k="
                         },
                         {
                           "pubKey": "5Giu-tJpGXU0S9Av75iAv5qDO0k6l8v_k9-UCcRUCl4="
                         },
                         {
                           "pubKey": "MnQxlmmKDybku4CfxsQQxfftilsaphF9Gq1w3MB1ZCE="
                         },
                         {
                           "pubKey": "GdP1fHVs2R65EkuWVWKZSz6WPDh0MgThyuBOv6_xsmQ="
                         },
                         {
                           "pubKey": "4JEtSrKsonmBuDvxJ9nITSu7iC4f8reutXRAVugPgS4="
                         },
                         {
                           "pubKey": "q4XyuUJbSGJaoRb3SmWzeX88V2dKB7sPTf72BAtQp3k="
                         },
                         {
                           "pubKey": "gTzKd1Ph5bpUw-JxTZiCe8RBfO-FsZiVYDYioQ-6dVg="
                         },
                         {
                           "pubKey": "NX4oaDLYEOmMUP_9pcEaZv5MJHJ4ZYAoxPQDxov7tRs="
                         },
                         {
                           "pubKey": "7fbk4oGQNlTW-tnWjVz8rWtKrtAicTsiNWgO98sqMyk="
                         },
                         {
                           "pubKey": "w1bfLpnd3rWu5JczB0nQfnE2S6nUCbx2AA7HDE48DQo="
                         }
                       ],
                       "lastSeen": 1575915097869025000,
                       "version": "0.1.0"
                     }
                   ]
                 }"#.to_string()
        }
    }
}

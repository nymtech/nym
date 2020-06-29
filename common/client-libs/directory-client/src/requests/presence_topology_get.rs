// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{DirectoryGetRequest, DirectoryRequest};
use crate::presence::Topology;

const PATH: &str = "/api/presence/topology";

pub struct Request {
    base_url: String,
    path: String,
}

impl DirectoryRequest for Request {
    fn url(&self) -> String {
        format!("{}{}", self.base_url, self.path)
    }
}

impl DirectoryGetRequest for Request {
    type JSONResponse = Topology;

    fn new(base_url: &str) -> Self {
        Request {
            base_url: base_url.to_string(),
            path: PATH.to_string(),
        }
    }
}

#[cfg(test)]
mod topology_requests {
    use super::*;
    use crate::client_test_fixture;
    use mockito::mock;

    #[cfg(test)]
    mod on_a_400_status {
        use super::*;

        #[tokio::test]
        async fn it_returns_an_error() {
            let _m = mock("GET", PATH)
                .with_status(400)
                .with_body("bad body")
                .create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client.get_topology().await;
            assert!(result.is_err());
            _m.assert();
        }
    }

    #[cfg(test)]
    mod on_a_200 {
        use super::*;

        #[tokio::test]
        async fn it_returns_a_response_with_200_status_and_a_correct_topology() {
            let json = fixtures::topology_response_json();
            let _m = mock("GET", PATH).with_status(200).with_body(json).create();
            let client = client_test_fixture(&mockito::server_url());
            let result = client.get_topology().await;
            assert_eq!(
                1_575_915_097_085_539_300,
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
                      "pubKey": "7fbk4oGQNlTW-tnWjVz8rWtKrtAicTsiNWgO98sqMyk="
                    },
                    {
                      "pubKey": "w1bfLpnd3rWu5JczB0nQfnE2S6nUCbx2AA7HDE48DQo="
                    }
                  ],
                  "lastSeen": 1575915097869025000,
                  "version": "0.1.0"
                }
              ],
              "gatewayNodes": [
                {
                  "location": "unknown",
                  "clientListener": "3.8.176.11:8888",
                  "mixnetListener": "3.8.176.11:9999",
                  "identityKey": "B9xz9V6jpp1fEbDkeyR5f8miorw9bzXGKoMbKnaxkD41",
                  "sphinxKey": "3KCpz1HCD8DqnQjemT1uuBZipmHFXM4V5btxLXwvM1gG",
                  "registeredClients": [
                    {
                      "pubKey": "zOqdJFH49HcgGSCRnmbXGzovnwRLEPN0YGN1SCafTyo="
                    },
                    {
                      "pubKey": "fy9xo69hZ2UJ9uxhIS1YzKHZsH8saV-02AiyCNXPNUc="
                    }
                  ],
                  "lastSeen": 1575915097358694100,
                  "version": "0.1.0"
                },
                {
                  "location": "unknown",
                  "clientListener": "3.8.176.12:8888",
                  "mixnetListener": "3.8.176.12:9999",
                  "identityKey": "CdqJCedY5d1geJNDjUqnEx8zF7mKjb6PCZ6k3T6xhxD",
                  "sphinxKey": "BnLYqQjb8K6TmW5oFdNZrUTocGxa3rgzBvapQrf8XUbF",
                  "registeredClients": [
                    {
                      "pubKey": "UE-7r6-bpw0b4T3GxOBVxlg02psx23DF2p5Tuf-OBSE="
                    },
                    {
                      "pubKey": "UnZuLpzq64_EPtIcr1Fd-5AESBCBLFnDMDsjUaOqrUA="
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

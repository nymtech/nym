import * as React from 'react';
import { ComponentMeta } from '@storybook/react';
import { Box } from '@mui/material';
import { ServiceProviderSelector } from './ServiceProviderSelector';
import { Services } from '../types/directory';

export default {
  title: 'Components/Service Provider Selector',
  component: ServiceProviderSelector,
} as ComponentMeta<typeof ServiceProviderSelector>;

const width = 240;

export const Loading = () => (
  <Box width={width}>
    <ServiceProviderSelector />
  </Box>
);

const services: Services = JSON.parse(`[
  {
    "id": "keybase",
    "description": "Keybase",
    "items": [
      {
        "id": "nym-keybase",
        "description": "Nym Keybase Service Provider",
        "address": "Entztfv6Uaz2hpYHQJ6JKoaCTpDL5dja18SuQWVJAmmx.Cvhn9rBJw5Ay9wgHcbgCnVg89MPSV5s2muPV2YF1BXYu@Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf",
        "gateway": "Fo4f4SQLdoyoGkFae5TpVhRVoXCF8UiypLVGtGjujVPf"
      },
      {
        "id": "shipyard-keybase-1",
        "description": "Nym Keybase Service Provider",
        "address": "D55ksecHzY6vAeqk8MCTzCfj2pqwJeKCKZCUUGnwGnn3.FS42vXS5a6GNTb1qk3aVk5mjSiJCAuawbBVyQZZVfhvt@DfNMqQRy6pPkU8Z5rBsxRwzDUzAMXHPFwMhjF16ScZqn",
        "gateway": "DfNMqQRy6pPkU8Z5rBsxRwzDUzAMXHPFwMhjF16ScZqn"
      },
      {
        "id": "shipyard-keybase-2",
        "description": "Nym Keybase Service Provider",
        "address": "DFdDtW7LNBATxQ4ef3jNbqs3cRE8b9wDZTCctHCQRULa.4AbKiTNVUwYFWHhy98o5pT9dELiUrkXoJQ9wHqPgf6GV@GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN",
        "gateway": "GJqd3ZxpXWSNxTfx7B1pPtswpetH4LnJdFeLeuY5KUuN"
      },
      {
        "id": "shipyard-keybase-3",
        "description": "Nym Keybase Service Provider",
        "address": "6Y1HE1jJ92P9yoHer11TR4A2NdZePrLGaBNFf65MnYGe.FwXoh217odQDWNmViqzNX28fauYrjB3PYLrVvpqnQrX4@5vC8spDvw5VDQ8Zvd9fVvBhbUDv9jABR4cXzd4Kh5vz",
        "gateway": "5vC8spDvw5VDQ8Zvd9fVvBhbUDv9jABR4cXzd4Kh5vz"
      },
      {
        "id": "shipyard-keybase-4",
        "description": "Nym Keybase Service Provider",
        "address": "3zzhLtWvaJgn755MkRckG5aRnoTZich8ASn395iSsTgj.J1R5VuxXbh2eNHiaRbrwbKGXrrEQcHKLdzf8eg9HTB6q@3B7PsbXFuqq6rerYFLw5HPbQb4UmBqAhfWURRovMmWoj",
        "gateway": "3B7PsbXFuqq6rerYFLw5HPbQb4UmBqAhfWURRovMmWoj"
      },
      {
        "id": "shipyard-keybase-5", 
        "description": "Nym Keybase Service Provider", 
        "address": "CHuXdZJYQ8xH7ekgN9gAuVtQ7ZikjjHEZY5BSN7yc5mN.29dFvqicKQQQvoX1vup44mspmc249RH5xgLibWMwTYGT@CfWcDJq8QBz6cVAPCYSaLbaJEhVTmHEmyYgQ6C5GdDW9", 
        "gateway": "CfWcDJq8QBz6cVAPCYSaLbaJEhVTmHEmyYgQ6C5GdDW9"
      }
    ]
  },
  {
    "id": "electrum",
    "description": "Electrum Wallet",
    "items": [
      {
        "id": "nym-electrum",
        "description": "Nym Electrum Service Provider",
        "address": "DpB3cHAchJiNBQi5FrZx2csXb1mrHkpYh9Wzf8Rjsuko.ANNWrvHqMYuertHGHUrZdBntQhpzfbWekB39qez9U2Vx@2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh",
        "gateway": "2BuMSfMW3zpeAjKXyKLhmY4QW1DXurrtSPEJ6CjX3SEh"
      },
      {
        "id": "shipyard-electrum-1",
        "description": "Nym Electrum Service Provider",
        "address": "8Tb73cFQpXCLpgxEA2VSDru2hHrcZ3KQcyMsGbxcTjBp.4x5tu66k8YkHk4tYac1qwEFbNq5WsKiX5kR51q5KKH88@4WgKhJdmUffz4e1o1ftVAGS3HnG56LiNAxA9dmaekrVd",
        "gateway": "4WgKhJdmUffz4e1o1ftVAGS3HnG56LiNAxA9dmaekrVd"
      },
      {
        "id": "shipyard-electrum-2",
        "description": "Nym Electrum Service Provider",
        "address": "GR6z31MwCsvxHrnvvVN1Cpasd8aQ1giwQqPTZM9dN7VH.5rEiqakSPDrBtKmvpU8Shnhz6gRM85JLoB7AX4h7PJYr@5Ao1J38frnU9Rx5YVeF5BWExcnDTcW8etNe9W2sRASXD",
        "gateway": "5Ao1J38frnU9Rx5YVeF5BWExcnDTcW8etNe9W2sRASXD"
      }   
  ]
  },
  {
    "id": "telegram",
    "description": "Telegram",
    "items": [
     {
        "id": "shipyard-telegram-2",
        "description": "Nym Telegram Service Provider",
        "address": "C4w6ewbQtoaZEeoaaNw1xVASChqo4WVjNfuYEUFjZxpc.8F1D7rQXf2jGoj1Ken7PiGDM8HS2Ug79wSoc9nZ1iqh1@62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve",
        "gateway": "62F81C9GrHDRja9WCqozemRFSzFPMecY85MbGwn6efve"
      },
      {
        "id": "shipyard-telegram-3",
        "description": "Nym Telegram Service Provider",
        "address": "DStL3BEUZuQZfbij1KAY3BvJh8rC5jpr9mc6AQ6aTLUu.Ax9foYaKfFgX6g8y393GoNpKkKrnDGFGRZwxDv9R7X6M@FQon7UwF5knbUr2jf6jHhmNLbJnMreck1eUcVH59kxYE",
        "gateway": "FQon7UwF5knbUr2jf6jHhmNLbJnMreck1eUcVH59kxYE"
      },
      {
        "id": "shipyard-telegram-4",
        "description": "Nym Telegram Service Provider",
        "address": "8gRdGTzsDxYzpasRQhsRg59MCgNNhnfag2oFfwwZPXnB.DtDrGz7ScVm4o7sN4K3CYUJveYgz7fcXELBVLNDfMS9Y@3ojQD6V7skM1bSXJX7fVQvscjmcgptzdixQEaAha2ixh",
        "gateway": "3ojQD6V7skM1bSXJX7fVQvscjmcgptzdixQEaAha2ixh"
      },
      {
        "id": "shipyard-telegram-5",
        "description": "Nym Telegram Service Provider",
        "address": "AR3oEM6Uvmfs6fyddwSehoBUKCFxz7MdFi4z7aahuHuY.3ZKapg9A3Py1PXhyLbCJr8ZbJsEV6NZdN1WJaGGut5tj@EEyq16v63aotPBCepxUpCgAojrNasZ6Hk1PjpRyBAdEp",
        "gateway": "EEyq16v63aotPBCepxUpCgAojrNasZ6Hk1PjpRyBAdEp"
      },
      {
        "id": "shipyard-telegram-6",
        "description": "Nym Telegram Service Provider",
        "address": "7n1BYhsXSwcr8Qim8AqZTAodqFia4QG6T7CRc1ihQHpv.7o4trpGqu2LHMUiXc3dddgfGET1CFFcAK9gKYoHoSn5e@BTZNB3bkkEePsT14GN8ofVtM1SJae4YLWjpBerrKYfr",
        "gateway": "BTZNB3bkkEePsT14GN8ofVtM1SJae4YLWjpBerrKYfr"
      },
      {
        "id": "shipyard-telegram-7",
        "description": "Nym Telegram Service Provider",
        "address": "Gv4TWhUKrvJfqh1jBRPGEQrikNZvZse2kS3ZgN9Z2nAZ.7KGPaaqUEum2C59jLvw7f8Ug8a48YuZdjjZu3t4JES4U@C7J8SwZQqjWqhBryyjJxLt7FacVuPTwAmR2otGy53ayi",
        "gateway": "C7J8SwZQqjWqhBryyjJxLt7FacVuPTwAmR2otGy53ayi"
      },
      {
        "id": "shipyard-telegram-8",
        "description": "Nym Telegram Service Provider",
        "address": "8Mqgp12cpF6FSXMeqzxgFgQXvTSapyNqGAi5wy7ub4ge.7z7PDsiJGiGxGz4p77v5L5fZhXBJ5qNZ8CgJwYNr6H6J",
        "gateway": "3zd3wrCK8Dz5TXrcvk5dG5s9EEdf4Ck1v9VgBPMMFVkR"
      },
      {
        "id": "shipyard-telegram-9",
        "description": "Nym Telegram Service Provider",
        "address": "F3N5eiPDZcGFC985Go4Mpv8p9uxFD1L3jRUdrLCbrZLm.EyTxWwwTwYpPrJBmc97GLd1LpUAphjptS5y1ed182bGk@GAjhJcrd6f1edaqUkfWCff6zdHoqo756qYrc2TfPuCXJ",
        "gateway": "GAjhJcrd6f1edaqUkfWCff6zdHoqo756qYrc2TfPuCXJ"
      },
      {
        "id": "shipyard-telegram-10",
        "description": "Nym Telegram Service Provider",
        "address": "G7y7e1nVBr8fmQSzdeAxXnCmmmJb5k8N3E8LBV31KE5g.GRRUCj6t6cCUUjakmTWzidMLiYA7EdCedKnup8osaBC6@AJad2R9virYEYXEsTcicN5y5tyPoixrhhAGsxoESZVnc",
        "gateway": "AJad2R9virYEYXEsTcicN5y5tyPoixrhhAGsxoESZVnc"
      },
      {
        "id": "shipyard-telegram-11",
        "description": "Nym Telegram Service Provider",
        "address": "2kq9Z7RyDZtb8kxXjyP3ZT8VMWHg6JXFDChGuuNBk7Hw.F5XYbBaGSoF8qAo8faPcaNRPHEq3Y25BDcwESeabUS9S@HaLyPQrhBTq75dnGeBUdYWeFVA2BBn39MgkhEt3VTMMM",
        "gateway": "HaLyPQrhBTq75dnGeBUdYWeFVA2BBn39MgkhEt3VTMMM"
      },
      {
        "id": "shipyard-telegram-12",
        "description": "Nym Telegram Service Provider",
        "address": "GegdtpNzYj4QCgpih9Kxv7ZVZxmVdxYHsDkiPsbT71XG.E8xtE8mrapjzFtyuziZSrsScAKhwZMH5wNpKWtKfzJ5Y@9Byd9VAtyYMnbVAcqdoQxJnq76XEg2dbxbiF5Aa5Jj9J",
        "gateway": "9Byd9VAtyYMnbVAcqdoQxJnq76XEg2dbxbiF5Aa5Jj9J"
      },
      {
        "id": "shipyard-telegram-13",
        "description": "Nym Telegram Service Provider",
        "address": "4SsrDQeEtG3mpeD9nN5CDdGaCsxFvNeYMhoviDzNNB9f.GyqG6iK5rBvhe3HXLR11m6ULpf13ATgYvkkidLmteDLs@5EpkkrMFYAM3XcaztXnZwBWquURHSKsyc9JxUCengDFS",
        "gateway": "5EpkkrMFYAM3XcaztXnZwBWquURHSKsyc9JxUCengDFS"
      },
      {
        "id": "shipyard-telegram-14",
        "description": "Nym Telegram Service Provider",
        "address": "9JoHRu2RrSD1fjbj9NSTASgjv9Szep7Nhd9L2PywxbBi.AZhAUDNX6iH8BqXyR5c7TJuzpwMEvDXrabNLGuRukvVf@9xJM74FwwHhEKKJHihD21QSZnHM2QBRMoFx9Wst6qNBS",
        "gateway": "9xJM74FwwHhEKKJHihD21QSZnHM2QBRMoFx9Wst6qNBS"
      },
      {
        "id": "shipyard-telegram-15",
        "description": "Nym Telegram Service Provider",
        "address": "3K174ijjXqCkhMDT9xLcqjS4MXk2QsqZt4PdgNcuUrnn.BNnHnQmWoj6Uo6kkS1QkPqsdHaXrcwyR9F6MnnzDkZJL@C7J8SwZQqjWqhBryyjJxLt7FacVuPTwAmR2otGy53ayi",
        "gateway": "C7J8SwZQqjWqhBryyjJxLt7FacVuPTwAmR2otGy53ayi"
      },
      {
        "id": "shipyard-telegram-16",
        "description": "Nym Telegram Service Provider",
        "address": "BqX5Q3MEcbTnM39hUswQchLW68SrqbhL8K5ucrLmtP39.AWrVsFoVC9s6KjdpcasATmZPA3GtMsUxcfHpAkuNdtFG@Emswx6KXyjRfq1c2k4d4uD2e6nBSbH1biorCZUei8UNS",
        "gateway": "Emswx6KXyjRfq1c2k4d4uD2e6nBSbH1biorCZUei8UNS"
      },
      {
        "id": "shipyard-telegram-17",
        "description": "Nym Telegram Service Provider",
        "address": "2tQxccgcqdkuUvLqgiEkEN4rNRZ5QknygnKAFcS4gfoe.EVrY5q5sqDqBUbS3wHsRRZhk2MAw1S17hNoH1Bicyv7n@DAGQxdxwAkwjaLjTw1B9vndia4YyFD15qRgcTQxrmkom",
        "gateway": "DAGQxdxwAkwjaLjTw1B9vndia4YyFD15qRgcTQxrmkom"
      },
      {
        "id": "shipyard-telegram-18",
        "description": "Nym Telegram Service Provider",
        "address": "8YG1rcEauJA814Nd7hSxjNe2UrRwrGsrXTm1Cmd3gRrU.FxYaYqpNN8PciNsySs3zYPrTB1J8AYUu9DBsM2vVDDfF@7EfEESLo71GUvx3UEW79LgTegHUBPUocUzGyJVv6LHog",
        "gateway": "7EfEESLo71GUvx3UEW79LgTegHUBPUocUzGyJVv6LHog"
      },
      {
        "id": "shipyard-telegram-19",
        "description": "Nym Telegram Service Provider",
        "address": "HPiXADVFLwLQPNpPtyYefzvYntC6tp9UJ5fJZGfkqvDt.2EUUxmeT3AiaUzAcE5SyXRAk8a2JXBkRz4B8McSdkrST@9ACTkYraCqE9jMb6zb6ne8EDQGGhZw5ykNiq9YRUdHTD",
        "gateway": "9ACTkYraCqE9jMb6zb6ne8EDQGGhZw5ykNiq9YRUdHTD"
      },
      {
        "id": "shipyard-telegram-20",
        "description": "Nym Telegram Service Provider",
        "address": "2QLnEEnTmf2NRWtcQPWBeRcg7Hej5WSPWRWwtTpEEZWF.BheS78ozc8ngvhsXNNnshdJzpoYsmEvhfn3WKUYF5dRU@C2uyokSPoxhku9GexRxEo1e8KPZ7q6e8FXmK3gtY8kkF",
        "gateway": "C2uyokSPoxhku9GexRxEo1e8KPZ7q6e8FXmK3gtY8kkF"
      },
      {
        "id": "shipyard-telegram-21",
        "description": "Nym Telegram Service Provider",
        "address": "FuBbnwiANfaXZnn683jBapK5XVm5ttgZSykU3vqPSHoD.94MFGv1VcBLTkRwzBDQUkWjvqtZYVBrJg2Q8JGbizcib@CTqYPY8htdAQMXCzRW9SjZzZuqYwSt2iUh6HPaNgmTvK",
        "gateway": "CTqYPY8htdAQMXCzRW9SjZzZuqYwSt2iUh6HPaNgmTvK"
      },
      {
        "id": "shipyard-telegram-22",
        "description": "Nym Telegram Service Provider",
        "address": "9EbQx5jQznSVbftFom7sqUSHAACrsfvMhrzhaFt4A3SZ.D1FQCirL4YKwfcmtMGvB5Rugt5sAzGEhdSjJ3bHVQRZ@7Zh1Sz5dXpA6s53CbtcdqhQhLqwf4cLynL7KqHKcjrG4",
        "gateway": "7Zh1Sz5dXpA6s53CbtcdqhQhLqwf4cLynL7KqHKcjrG4"
      },
      {
        "id": "shipyard-telegram-24",
        "description": "Nym Telegram Service Provider",
        "address": "6Umawwvf551VyB3Ko46NgKLqJdTFJeToCM67mrTmM3G.3A4sesBac4KGuMTFjvYBwLpksMJvbMbteGJQgmm4PV4Y@AnnYnEtBjB2a5sHmeRCnBq43qxyHDf95Bqd7cwQyKNLR",
        "gateway": "AnnYnEtBjB2a5sHmeRCnBq43qxyHDf95Bqd7cwQyKNLR"
      },
      {
        "id": "shipyard-telegram-25",
        "description": "Nym Telegram Service Provider",
        "address": "CDtxTeoyqq83JpV9G8cR5HRHRdMMaVspQsCwH3Qnajt3.F5EHK9HFcdGrE2hqA7bK9AUmkbihujYDhtNNqHKxW765@BDkeNx7JQm5NsQakst9s8htogZXhpTQedFAgZpvsGCqH",
        "gateway": "BDkeNx7JQm5NsQakst9s8htogZXhpTQedFAgZpvsGCqH"
      },
      {
        "id": "shipyard-telegram-26",
        "description": "Nym Telegram Service Provider",
        "address": "HukZkLG2DoarQEqaoDLuqW1GFf2NSHDUMGBZiyJGRYJD.9GyU8wPsyzcvRjcyk8hiNpTJbXCmq5F3VoVhFBZYuHR3@GsGEZiDBz8SWfHGaK5SDmhfbTEM55v37WCYYcT9wTSxN",
        "gateway": "GsGEZiDBz8SWfHGaK5SDmhfbTEM55v37WCYYcT9wTSxN"
      },
      {
        "id": "shipyard-telegram-27", 
        "description": "Nym Telegram Service Provider", 
        "address": "773y8iMVJiRk4dRbjQzkJVbrei4TwkePNE5WTEttt77d.3Mw47C9XZj3oAzk1iSqC5Y36tbBsjtaTtdgaHM3Zsdma@7fiZtNL1RACQTwGrKLBT9nbY77bfwZnX9rqcWqc53qgv",
        "gateway": "7fiZtNL1RACQTwGrKLBT9nbY77bfwZnX9rqcWqc53qgv"
      }, 
      {
        "id": "shipyard-telegram-28", 
        "description": "Nym Telegram Service Provider", 
        "address": "6jQJEorCu7YiP9HdDaMeHxcNhxeNmZ1kpd836GnqLZX.HsJqEmNTszGecsKqFB37i84nBXxqf4ETgrKmKmBvMGHC@FYnDMQzT49ZGM23gVqpTxfih14V6wuedNXirekmt37zE", 
        "gateway": "FYnDMQzT49ZGM23gVqpTxfih14V6wuedNXirekmt37zE"
      }, 
      {
        "id": "shipyard-telegram-29", 
        "description": "Nym Telegram Service Provider", 
        "address": "BiCSyovpFMuSnTvF2TdiuZwrytXDrd9AH47ZMcCxscVC.G9YpdicA9BBNoVHDgjWjgt17wv5WYKWcbE3vPJJVpSJD@GAjhJcrd6f1edaqUkfWCff6zdHoqo756qYrc2TfPuCXJ", 
        "gateway":"GAjhJcrd6f1edaqUkfWCff6zdHoqo756qYrc2TfPuCXJ" 
      }, 
      {
        "id": "shipyard-telegram-30", 
        "description": "Nym Telegram Service Provider", 
        "address": "AQRRAs9oc8QWXAFBs44YhCKUny7AyLsfLy91pwmGgxuf.CWUKoKA1afSKyw5BnFJJg19UDgnaVATupsFhQpyTEBHJ@EBT8jTD8o4tKng2NXrrcrzVhJiBnKpT1bJy5CMeArt2w", 
        "gateway": "EBT8jTD8o4tKng2NXrrcrzVhJiBnKpT1bJy5CMeArt2w"
      }, 
      {
        "id": "shipyard-telegram-31", 
        "description": "Nym Telegram Service Provider", 
        "address": "6YqjAZK3Pr1ngiBLcDkotboB5WiN6k6NPpbXvShH4pR5.9Ss6VW3Xbyi8LuxduNNwnXEv9njHCQ2PLSP1UK6tsoa5@42XCK9dMS9m5XJLzQd2dBuwimk6ndZnczhZaV5VPFkQD",
        "gateway": "42XCK9dMS9m5XJLzQd2dBuwimk6ndZnczhZaV5VPFkQD" 
      }, 
      { 
        "id": "shipyard-telegram-32",
        "description": "Nym Telegram Service Provider", 
        "address": "EmYWLeybmj86Vzr62vxuZ3T15jwMNHggzK7sQwid96yp.GyaF9WprSr56cxUdGf5TpcUvAjb2VbAr8CVBrmBUYAaw@GL5wESoz4oSbpBaTki9qB9213FGUQXCiRjbzDkhWwoBC", 
        "gateway": "GL5wESoz4oSbpBaTki9qB9213FGUQXCiRjbzDkhWwoBC"
      }, 
      {
        "id": "shipyard-telegram-33", 
        "description": "Nym Telegram Service Provider", 
        "address": "4PDb96cck5btTj6G7rsomqwHJsp4qu8uPvFCbwHfjFUx.C5dKbaoakH7egsZvAueRbwLFbmxnQaVMeSr6QTMpuBAA@58ceEFaLJh6zAo3cirzT1BDQm7D3L5acnQrxGH1D6TAY", 
        "gateway": "58ceEFaLJh6zAo3cirzT1BDQm7D3L5acnQrxGH1D6TAY"
      }, 
      {
         "id": "shipyard-telegram-34", 
         "description": "Nym Telegram Service Provider", 
         "address": "BeZbeMf9vcpUf368qDd85dtLwXLj4Ee5bsHMB2fUD8uX.HELVbppkwU1jmzUAUrCEbHeJfVciSeo8VGAkbJSpwxsb@ADdHkiTfkpsSt31zVToWW9j3KikH24aLAAwDKtCYE5jY", 
         "gateway":"ADdHkiTfkpsSt31zVToWW9j3KikH24aLAAwDKtCYE5jY"
      }, 
      { 
        "id": "shipyard-telegram-35", 
        "description": "Nym Telegram Service Provider", 
        "address": "Bp4JRFyf7GB9L9J95AqMPnz9zbGmPnViA5fDXKeNraLJ.D6CTdcjJVxDmH2UQvzXuPWg9Se9xXYe76uDMypXvhzd7@6UjGEeQZK14C5K2kenycTkqt7qRjEHGLgaQx3FWySo3N", 
        "gateway": "6UjGEeQZK14C5K2kenycTkqt7qRjEHGLgaQx3FWySo3N"
      }, 
      { 
        "id": "shipyard-telegram-36", 
        "description": "Nym Telegram Service Provider", 
        "address": "91h7io6BGhVjbtC7dbbRcScyTJcTfnMsTQZ6aWMVsrWR.Epb4hANXCp8cGEY3wSgawux991ti9Z5Y1FHTMzAKFa6E@DF4TE7V8kJkttMvnoSVGnRFFRt6WYGxxiC2w1XyPQnHe", 
        "gateway": "DF4TE7V8kJkttMvnoSVGnRFFRt6WYGxxiC2w1XyPQnHe"
      }, 
      { 
        "id": "shipyard-telegram-37", 
        "description": "Nym Telegram Service Provider", 
        "address": "Cy2wuwKpWZ3iWzKU3ZWL1qqcVfJ5Cq85dU7UHVWwv2gc.9AhC9b2zVcLnXLGriMdxjpsWJpq6iAdCavDi63udbL89@678qVUJ21uwxZBhp3r56z7GRf6gMh3NYDHruTegPtgMf", 
        "gateway": "678qVUJ21uwxZBhp3r56z7GRf6gMh3NYDHruTegPtgMf"
      }, 
      { 
        "id": "shipyard-telegram-38", 
        "description": "Nym Telegram Service Provider", 
        "address": "GgUeUWW1NRSuquZYeZf3WkppE92EQUHJrFjNZtYU1jow.CSEjwrRi4f4uyw7N6L2LPKw2tB8spcMbFu99wHZzFZSj@77TSuVU8d1oXKbPzjec2xh4i3Wj5WwUyy9Lr36sm8gZm", 
        "gateway": "77TSuVU8d1oXKbPzjec2xh4i3Wj5WwUyy9Lr36sm8gZm"
      },
      { 
        "id": "shipyard-telegram-39", 
        "description": "Nym Telegram Service Provider", 
        "address": "kz4zWwSkYiQxqxXFPNcGUByTPQWXascD9RfYsmSxY7n.ajp3SjbBVBjrU9nXpSQXAXzbb6EHJJyhbY6cc1ajayx@BTZNB3bkkEePsT14GN8ofVtM1SJae4YLWjpBerrKYf", 
        "gateway": "HyS2UZtZX3kQXdazbdE99DhCjBXjbG61LC9QsmXwbxrU"
      }
    ]
  },
  {
    "id": "blockstream",
    "description": "Blockstream Green",
    "items": [
      {
        "id": "nym-blockstream",
        "description": "Nym Blockstream Green Service Provider",
        "address": "GiRjFWrMxt58pEMuusm4yT3RxoMD1MMPrR9M2N4VWRJP.3CNZBPq4vg7v7qozjGjdPMXcvDmkbWPCgbGCjQVw9n6Z@2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW",
        "gateway": "2xU4CBE6QiiYt6EyBXSALwxkNvM7gqJfjHXaMkjiFmYW"
      }
    ]
  }
]`);
export const Loaded = () => (
  <Box width={width}>
    <ServiceProviderSelector services={services} />
  </Box>
);

export const ServiceAlreadySelected = () => (
  <Box width={width}>
    <ServiceProviderSelector
      services={services}
      currentSp={services[2].items[2]}
      onChange={(serviceProvider) => console.log('New service provider selected: ', serviceProvider)}
    />
  </Box>
);

# Komunikator 

## Autorzy
- Julia Karmowska (gr 4, @jkarmowska na githubie)

## Opis
Klient i serwer czatu w Ruscie

## Funkcjonalność
- Logowanie użytkowników i ustawianie nicków [done]
- Wysyłanie wiadomości do grupy użytkowników [done]
- Połączenie przez TCP [done]

## Propozycja podziału na części
Pierwsza część - klient dołącza do jednej grupy na początku, grupy tworzone wraz z serwerem [na razie tylko jedna grupa]

Druga część - klient może być w kilku grupach, może opuścić grupę, stworzyć nową, wybrać do której grupy chce wysłać wiadomość

##  Todo
Serializacja wiadomości
Stworzenie kilku grup i wyboru do której user chce dołączyć

## Biblioteki
- Tokio - do obsługi współbieżności

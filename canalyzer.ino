#include <defaults.h>
#include <global.h>
#include <Canbus.h>
#include <mcp2515.h>
#include <mcp2515_defs.h>

void setup()
{
  Serial.begin(9600); // For debug use

  if (Canbus.init(CANSPEED_500)) // Initialise MCP2515 CAN controller at the specified speed
    Serial.println("CAN Init ok");
  else
    Serial.println("Can't init CAN");

  Serial.println("CAN Read - Test receiving of CAN Bus message");
  delay(1000);
}

void loop()
{
  tCAN message;
  if (mcp2515_check_message())
  {
    if (mcp2515_get_message(&message))
    {
      // Watch bus for PCM messages:
      //  MIL request - 
      //  Oil pressure warning indicator request - 
      Serial.print("ID: ");
      Serial.print(message.id, HEX);
      Serial.print(" ");
      Serial.print("Data: ");
      for (int i = 0; i < message.header.length; i++)
      {
        char data[3];
        sprintf(data, "%02X", message.data[i]);
        Serial.print(data);
      }
      Serial.println("");
    }
  }
}
